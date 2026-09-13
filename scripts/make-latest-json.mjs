/**
 * Builds the updater manifest (latest.json) for a GitHub release from the
 * locally built, signed NSIS bundle.
 *
 * Usage:  node scripts/make-latest-json.mjs <notes-file>
 *
 * Reads the version from src-tauri/tauri.conf.json, the signature from the
 * .exe.sig next to the setup bundle, and the release notes from the given
 * file (first paragraph). Requires verified publisher signatures, trusted
 * timestamps and matching updater signatures before preparing any output.
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import os from "node:os";
import { verifyUpdater } from "./verify-updater.mjs";

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");
const conf = JSON.parse(fs.readFileSync(path.join(root, "src-tauri", "tauri.conf.json"), "utf8"));
const version = conf.version;
const repoUrl = conf.plugins?.updater?.endpoints?.[0]?.match(
  /https:\/\/github\.com\/[^/]+\/[^/]+/,
)?.[0];
if (!repoUrl) {
  console.error("Could not derive the repo URL from the updater endpoint.");
  process.exit(1);
}

const nsisDir = path.join(root, "src-tauri", "target", "release", "bundle", "nsis");
const setups = fs
  .readdirSync(nsisDir)
  .filter((f) => f.includes(`_${version}_`) && f.endsWith("_x64-setup.exe"));
if (setups.length !== 1) throw new Error("Expected exactly one current x64 NSIS installer.");
const setup = setups[0];
if (!setup) {
  console.error(`No v${version} -setup.exe found in ${nsisDir}. Run the signed build first.`);
  process.exit(1);
}
const sigPath = path.join(nsisDir, setup + ".sig");
if (!fs.existsSync(sigPath)) {
  console.error(`${setup} has no updater .sig. Refusing to write latest.json.`);
  process.exit(1);
}

const notesFile = process.argv[2];
if (!notesFile) {
  console.error("Usage: node scripts/make-latest-json.mjs <notes-file>");
  process.exit(1);
}
// First paragraph of the notes file, flattened to one line.
const notes = fs
  .readFileSync(notesFile, "utf8")
  .split(/\n\s*\n/)
  .map((p) => p.replace(/^#.*$/gm, "").replace(/\s+/g, " ").trim())
  .filter(Boolean)[0];

// GitHub does not serve a release asset under the name it was uploaded
// with: every character outside [A-Za-z0-9.-_] becomes a dot, so
// "PC Tweaker Uninstaller_0.8.2_x64-setup.exe" is downloadable only as
// "PC.Tweaker.Uninstaller_0.8.2_x64-setup.exe". Percent-encoding the
// spaces instead produces a URL that 404s, which is how 0.8.0's winget
// manifest shipped pointing at an asset that was never there — and an
// updater manifest with a dead URL strands every install on the old
// version silently, since the check succeeds and the download does not.
const assetName = setup.replace(/[^A-Za-z0-9.\-_]/g, ".");
if (/[^A-Za-z0-9.\-_]/.test(assetName)) {
  console.error(`Asset name still holds characters GitHub will rewrite: ${assetName}`);
  process.exit(1);
}

const msiDir = path.join(root, "src-tauri", "target", "release", "bundle", "msi");
const msis = fs.readdirSync(msiDir).filter((f) => f.includes(`_${version}_`) && f.endsWith(".msi"));
if (msis.length !== 1) throw new Error("Expected exactly one current MSI installer.");
const msi = msis[0];
const releaseDir = path.join(root, "src-tauri", "target", "release");

/**
 * The application binary that actually ships, taken out of the MSI.
 *
 * Verifying `target/release/pc-tweaker-uninstaller.exe` looks right and is
 * wrong. Tauri patches that file with bundle-type information, signs it,
 * packages it, and then restores the pre-patch bytes when it is done - so
 * after a correct signed build the leftover in `target/release` is unsigned
 * while the copy inside both installers is signed. This helper refused to
 * write a manifest for a perfectly good release twice, on two products, and
 * both times the workaround was to copy the payload back by hand before
 * running it. That is a manual step in the middle of a signing procedure,
 * which is exactly where manual steps get skipped.
 *
 * `msiexec /a` is an administrative install: it unpacks the MSI without
 * installing anything, needs no elevation, and gives us the bytes that reach
 * the user. If it cannot run, fall back to the build output rather than
 * blocking a release - the installers themselves are still verified below,
 * and they are what is published.
 */
function shippedApplication(msiPath) {
  const staging = fs.mkdtempSync(path.join(os.tmpdir(), "pcu-payload-"));
  try {
    execFileSync("msiexec", ["/a", msiPath, "/qn", `TARGETDIR=${staging}`], { stdio: "ignore" });
    const found = [];
    const walk = (dir) => {
      for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
        const full = path.join(dir, entry.name);
        if (entry.isDirectory()) walk(full);
        else if (entry.name.toLowerCase() === "pc-tweaker-uninstaller.exe") found.push(full);
      }
    };
    walk(staging);
    if (found.length !== 1) throw new Error(`expected one payload executable, found ${found.length}`);
    // Copied out before the staging directory goes away, and left in place:
    // the verification below reads it, and a human comparing hashes after the
    // fact needs it to still be there.
    const destination = path.join(releaseDir, "pc-tweaker-uninstaller.shipped.exe");
    fs.copyFileSync(found[0], destination);
    return destination;
  } catch (error) {
    console.warn(`Could not unpack the MSI payload (${error.message}); verifying the build output instead.`);
    return path.join(releaseDir, "pc-tweaker-uninstaller.exe");
  } finally {
    fs.rmSync(staging, { recursive: true, force: true });
  }
}

const application = shippedApplication(path.join(msiDir, msi));
const binaries = [
  application,
  ...fs
    .readdirSync(releaseDir)
    .filter((f) => f.endsWith(".dll"))
    .map((f) => path.join(releaseDir, f)),
  path.join(nsisDir, setup),
  path.join(msiDir, msi),
];
const sha256 = (file) => createHash("sha256").update(fs.readFileSync(file)).digest("hex");
const hashes = new Map(binaries.map((file) => [file, sha256(file)]));
for (const binary of binaries) {
  execFileSync(
    "pwsh",
    [
      "-NoProfile",
      "-NonInteractive",
      "-File",
      path.join(root, "scripts", "verify-authenticode.ps1"),
      "-Path",
      binary,
    ],
    { stdio: "inherit" },
  );
}
const updaterSignature = verifyUpdater(path.join(nsisDir, setup), conf.plugins.updater.pubkey);
verifyUpdater(path.join(msiDir, msi), conf.plugins.updater.pubkey);
for (const [file, hash] of hashes) {
  if (sha256(file) !== hash) throw new Error(`File changed during preparation: ${file}`);
}

const manifest = {
  version,
  notes,
  pub_date: new Date().toISOString().replace(/\.\d{3}Z$/, "Z"),
  platforms: {
    "windows-x86_64": {
      signature: updaterSignature,
      url: `${repoUrl}/releases/download/v${version}/${assetName}`,
    },
  },
};

const out = path.join(nsisDir, "latest.json");
fs.writeFileSync(out, JSON.stringify(manifest, null, 2) + "\n");
console.log(`latest.json written for v${version}\n  bundle: ${setup}\n  out:    ${out}`);

// A copy under a name that carries no version number. GitHub only serves
// `releases/latest/download/<name>` for an exact filename, so a versioned
// asset has no permanent URL - and PC Tweaker's Uninstaller card links a
// permanent one so the button starts a download instead of dropping somebody
// on a repository page. Forget this copy on a release and that button 404s
// silently, which is why it is produced here rather than left to whoever is
// cutting the release. PC Tweaker has published PCTweaker-Setup.exe this way
// for a while; this is the same trick for the Uninstaller.
const stable = path.join(nsisDir, "PCTweakerUninstaller-Setup.exe");
fs.copyFileSync(path.join(nsisDir, setup), stable);

const assets = [
  path.join(nsisDir, setup),
  path.join(nsisDir, setup + ".sig"),
  ...(msi ? [path.join(msiDir, msi), path.join(msiDir, msi + ".sig")] : []),
  stable,
  out,
];
console.log(
  "\nUpload all of these. The stable-named copy is what the in-app download " +
    "button points at:\n\n  gh release upload v" +
    version +
    " \\\n" +
    assets.map((a) => `    "${a}"`).join(" \\\n") +
    "\n",
);
