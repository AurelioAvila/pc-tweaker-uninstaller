// `tauri build` refuses a @tauri-apps JavaScript package and its Rust crate on
// different major.minor versions ("Found version mismatched Tauri packages"),
// but the checks only ran `vite build`, so 0.13.10 found it in the signed
// release build (#80: plugin-process 2.4.0 in npm, 2.3.1 as a crate). This
// compares the two lockfiles the same way, in a second.
//   node scripts/check-tauri-versions.mjs
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

/** Every @tauri-apps/api and @tauri-apps/plugin-* package installed by npm,
 *  next to the tauri / tauri-plugin-* crate Cargo resolved for it. */
export function tauriPairs(packageLock, cargoLock) {
  const crates = new Map(
    [...cargoLock.matchAll(/\[\[package\]\]\r?\nname = "([^"]+)"\r?\nversion = "([^"]+)"/g)].map(
      (m) => [m[1], m[2]],
    ),
  );
  return Object.entries(packageLock.packages ?? {}).flatMap(([path, info]) => {
    const name = /^node_modules\/@tauri-apps\/(api|plugin-[\w-]+)$/.exec(path)?.[1];
    if (!name) return [];
    const crate = name === "api" ? "tauri" : `tauri-${name}`;
    return [{ npm: `@tauri-apps/${name}`, npmVersion: info.version, crate, crateVersion: crates.get(crate) ?? null }];
  });
}

const majorMinor = (version) => version?.split(".").slice(0, 2).join(".");

/** The pairs `tauri build` would refuse; a JS plugin without its crate counts. */
export function mismatches(pairs) {
  return pairs.filter((p) => majorMinor(p.npmVersion) !== majorMinor(p.crateVersion));
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  const root = new URL("../", import.meta.url);
  const pairs = tauriPairs(
    JSON.parse(readFileSync(new URL("package-lock.json", root), "utf8")),
    readFileSync(new URL("src-tauri/Cargo.lock", root), "utf8"),
  );
  for (const p of pairs) console.log(`${p.npm} ${p.npmVersion} / ${p.crate} ${p.crateVersion ?? "missing"}`);
  const bad = mismatches(pairs);
  if (pairs.length === 0 || bad.length > 0) {
    for (const p of bad) {
      console.error(`MISMATCH ${p.npm} ${p.npmVersion} vs ${p.crate} ${p.crateVersion ?? "(no crate)"}: move the npm side to the crate's major.minor`);
    }
    if (pairs.length === 0) console.error("no @tauri-apps packages found in package-lock.json");
    process.exit(1);
  }
  console.log(`ok: ${pairs.length} Tauri JavaScript packages on their crates' major.minor`);
}
