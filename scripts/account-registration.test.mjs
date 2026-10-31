import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { transform } from "esbuild";

async function loadAccount() {
  const source = (await readFile(new URL("../src/account.ts", import.meta.url), "utf8"))
    .replace('import { invoke } from "@tauri-apps/api/core";', "const invoke = async () => {}; ")
    .replace("import.meta.env", "({ VITE_API_BASE_URL: undefined })");
  const compiled = await transform(source, { loader: "ts", format: "esm" });
  return import(`data:text/javascript,${encodeURIComponent(compiled.code)}`);
}

test("a rate-limited sign-in says how long to wait when the service sends Retry-After", async () => {
  const { login, retryAfterSeconds, AccountRequestError } = await loadAccount();
  const now = Date.parse("2026-10-31T12:00:00Z");
  assert.equal(retryAfterSeconds("120", now), 120);
  assert.equal(retryAfterSeconds(" 45 ", now), 45);
  assert.equal(retryAfterSeconds("Sat, 31 Oct 2026 12:03:00 GMT", now), 180);
  // Absent, unreadable, already past or absurdly far: the general message.
  for (const header of [null, "", "soon", "0", "-5", "1.5", "Sat, 31 Oct 2026 11:00:00 GMT", "999999"]) {
    assert.equal(retryAfterSeconds(header, now), undefined, String(header));
  }

  const previousFetch = globalThis.fetch;
  const previousStorage = globalThis.localStorage;
  globalThis.localStorage = { getItem: () => null, setItem: () => {}, removeItem: () => {} };
  const answer = (status, retryAfter) => async () => ({
    ok: false,
    status,
    headers: { get: (name) => (name.toLowerCase() === "retry-after" ? retryAfter : null) },
    json: async () => ({}),
  });
  const failure = async () => {
    try {
      await login("a@example.com", "password123");
    } catch (error) {
      return error;
    }
    assert.fail("login should have failed");
  };
  try {
    globalThis.fetch = answer(429, "300");
    let error = await failure();
    assert.ok(error instanceof AccountRequestError);
    assert.equal(error.key, "errorRateLimited");
    assert.equal(error.retryAfter, 300);
    // A service that does not send (or expose) the header: unchanged message.
    globalThis.fetch = answer(429, null);
    error = await failure();
    assert.equal(error.key, "errorRateLimited");
    assert.equal(error.retryAfter, undefined);
    // Only a 429 carries a wait.
    globalThis.fetch = answer(401, "300");
    error = await failure();
    assert.equal(error.key, "errorCredentials");
    assert.equal(error.retryAfter, undefined);
  } finally {
    globalThis.fetch = previousFetch;
    globalThis.localStorage = previousStorage;
  }
});

test("registration uses the suite API, persists its session and adopts a renewed token", async () => {
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
    const headers = (token) => ({ get: (name) => (name === "X-Session-Token" ? token : null) });
    return url.endsWith("/api/auth/register")
      ? { ok: true, headers: headers(null), json: async () => ({ token: "signed-token", verificationEmailSent: true }) }
      : { ok: true, headers: headers("renewed-token"), json: async () => ({ email: "new@example.com", isPro: false, emailVerified: false }) };
  };
  try {
    const { register } = await import(`data:text/javascript,${encodeURIComponent(compiled.code)}`);
    const result = await register("New@Example.com", "password123", "New", "User", "1990-01-01");
    assert.equal(result.account.status, "signed-in");
    assert.equal(result.verificationEmailSent, true);
    assert.equal(entries.get("pcu-token"), "renewed-token");
    assert.equal(entries.get("pcu-email"), "new@example.com");
    assert.equal(calls.length, 2);
    const { deviceId, ...body } = JSON.parse(calls[0].options.body);
    assert.deepEqual(body, {
      email: "new@example.com", password: "password123", firstName: "New",
      lastName: "User", dateOfBirth: "1990-01-01", product: "uninstaller", locale: "en",
    });
    assert.match(deviceId, /^[0-9a-f-]{36}$/, "a random installation id, kept for later sign-ins");
    assert.equal(entries.get("pcu-installation-id"), deviceId);
    assert.equal(calls[1].options.headers.Authorization, "Bearer signed-token");
  } finally {
    globalThis.fetch = previousFetch;
    globalThis.localStorage = previousStorage;
  }
});
