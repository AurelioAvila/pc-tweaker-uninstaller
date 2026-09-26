import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { transform } from "esbuild";

test("registration uses the suite API and persists its returned session", async () => {
  const source = (await readFile(new URL("../src/account.ts", import.meta.url), "utf8"))
    .replace('import { invoke } from "@tauri-apps/api/core";', "const invoke = async () => {}; ")
    .replace("import.meta.env", "({ VITE_API_BASE_URL: undefined })");
  const compiled = await transform(source, { loader: "ts", format: "esm" });
  const previousFetch = globalThis.fetch;
  const previousStorage = globalThis.localStorage;
  const entries = new Map();
  const calls = [];
  globalThis.localStorage = {
    getItem: (key) => entries.get(key) ?? null,
    setItem: (key, value) => entries.set(key, value),
    removeItem: (key) => entries.delete(key),
  };
  globalThis.fetch = async (url, options) => {
    calls.push({ url, options });
    return url.endsWith("/api/auth/register")
      ? { ok: true, json: async () => ({ token: "signed-token", verificationEmailSent: true }) }
      : { ok: true, json: async () => ({ email: "new@example.com", isPro: false, emailVerified: false }) };
  };
  try {
    const { register } = await import(`data:text/javascript,${encodeURIComponent(compiled.code)}`);
    const result = await register("New@Example.com", "password123", "New", "User", "1990-01-01");
    assert.equal(result.account.status, "signed-in");
    assert.equal(result.verificationEmailSent, true);
    assert.equal(entries.get("pcu-token"), "signed-token");
    assert.equal(entries.get("pcu-email"), "new@example.com");
    assert.equal(calls.length, 2);
    assert.deepEqual(JSON.parse(calls[0].options.body), {
      email: "New@Example.com", password: "password123", firstName: "New",
      lastName: "User", dateOfBirth: "1990-01-01",
    });
    assert.equal(calls[1].options.headers.Authorization, "Bearer signed-token");
  } finally {
    globalThis.fetch = previousFetch;
    globalThis.localStorage = previousStorage;
  }
});
