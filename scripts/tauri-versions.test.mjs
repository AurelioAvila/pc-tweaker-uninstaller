import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { mismatches, tauriPairs } from "./check-tauri-versions.mjs";

const lock = (packages) => ({
  packages: Object.fromEntries(
    Object.entries(packages).map(([name, version]) => [`node_modules/@tauri-apps/${name}`, { version }]),
  ),
});
const cargo = (crates) =>
  Object.entries(crates)
    .map(([name, version]) => `[[package]]\nname = "${name}"\nversion = "${version}"\nsource = "x"\n`)
    .join("\n");

test("a JS plugin a minor ahead of its crate is caught, as #80 was", () => {
  const pairs = tauriPairs(
    lock({ api: "2.12.4", "plugin-process": "2.4.0", "plugin-updater": "2.14.1", cli: "2.12.4" }),
    cargo({ tauri: "2.12.5", "tauri-plugin-process": "2.3.1", "tauri-plugin-updater": "2.14.1" }),
  );
  assert.deepEqual(
    pairs.map((p) => p.npm),
    ["@tauri-apps/api", "@tauri-apps/plugin-process", "@tauri-apps/plugin-updater"],
    "the CLI is not paired with a crate",
  );
  assert.deepEqual(
    mismatches(pairs).map((p) => p.crate),
    ["tauri-plugin-process"],
  );
});

test("patch differences pass; a plugin without its crate does not", () => {
  const pairs = tauriPairs(
    lock({ api: "2.12.4", "plugin-opener": "2.5.9" }),
    cargo({ tauri: "2.12.0", "tauri-plugin-opener": "2.5.0" }),
  );
  assert.equal(mismatches(pairs).length, 0);
  const orphan = tauriPairs(lock({ "plugin-shell": "2.3.0" }), cargo({ tauri: "2.12.0" }));
  assert.equal(mismatches(orphan).length, 1);
});

test("this repository's lockfiles agree", () => {
  const root = new URL("../", import.meta.url);
  const pairs = tauriPairs(
    JSON.parse(readFileSync(new URL("package-lock.json", root), "utf8")),
    readFileSync(new URL("src-tauri/Cargo.lock", root), "utf8"),
  );
  assert.ok(pairs.length >= 4, "api and the three plugins");
  assert.deepEqual(mismatches(pairs), []);
});
