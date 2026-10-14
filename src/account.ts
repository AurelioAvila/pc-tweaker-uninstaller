/**
 * Suite account: login against the SAME backend PC Tweaker uses, so a
 * PC Tweaker Pro subscription is what actually gates loyalty pricing and any
 * Pro perk in this app — never local detection of PC Tweaker being
 * installed. Installed-on-this-PC is a hint shown as a badge; it proves
 * nothing about the account, and nothing here treats it as if it did.
 *
 * New users can register against the same suite backend before checkout.
 */

import { invoke } from "@tauri-apps/api/core";
export const API_BASE_URL: string =
  (import.meta.env.VITE_API_BASE_URL as string | undefined) ??
  "https://pc-tweaker-app-production.up.railway.app";

const TOKEN_KEY = "pcu-token";
const EMAIL_KEY = "pcu-email";

export type AccountState =
  | { status: "anonymous" }
  | { status: "checking"; email: string }
  | {
      status: "signed-in";
      email: string;
      isPro: boolean;
      emailVerified: boolean;
      /** A Stripe customer exists, so "Manage subscription" has something to open. */
      hasBilling: boolean;
    }
  | { status: "error"; email: string; message: string };

export function readStoredEmail(): string | null {
  try {
    return localStorage.getItem(EMAIL_KEY);
  } catch {
    return null;
  }
}

function readToken(): string | null {
  try {
    return localStorage.getItem(TOKEN_KEY);
  } catch {
    return null;
  }
}

function storeSession(token: string, email: string): void {
  try {
    localStorage.setItem(TOKEN_KEY, token);
    localStorage.setItem(EMAIL_KEY, email);
  } catch {
    // Session just won't survive a restart; login itself still worked.
  }
}

export function clearSession(): void {
  try {
    localStorage.removeItem(TOKEN_KEY);
    localStorage.removeItem(EMAIL_KEY);
  } catch {
    // Nothing to do.
  }
}

/** Why an account request failed, as a key of the `menu` dictionary. The
 *  server's own text is English, and a rate-limited request has none. */
export type AccountErrorKey =
  | "errorCurrentPassword"
  | "errorCredentials"
  | "errorAlreadyRegistered"
  | "errorRateLimited"
  | "errorInvalid"
  | "errorServer"
  | "errorNetwork";
export class AccountRequestError extends Error {
  constructor(readonly key: AccountErrorKey) {
    super(key);
  }
}
function failure(status: number): AccountRequestError {
  return new AccountRequestError(
    status === 401
      ? "errorCredentials"
      : status === 409
        ? "errorAlreadyRegistered"
        : status === 429
          ? "errorRateLimited"
          : status >= 500
            ? "errorServer"
            : "errorInvalid",
  );
}
async function request(path: string, init: RequestInit): Promise<Response> {
  try {
    return await fetch(`${API_BASE_URL}${path}`, init);
  } catch {
    throw new AccountRequestError("errorNetwork");
  }
}
/** The account is shared with PC Tweaker and Tweaky Driver; mail about it is
 *  branded for the app that asked, and written in the app's language. */
const PRODUCT = "uninstaller";
const INSTALLATION_KEY = "pcu-installation-id";

/** The app's language, which account mail is written in. Set by the app
 *  whenever it changes, so this module does not import the dictionaries. */
let mailLocale = "en";
export function setAccountLocale(locale: string): void {
  mailLocale = locale;
}

/** A random id this installation keeps, sent at sign-in so the account can
 *  tell a new PC from a known one. Not a hardware fingerprint. */
function installationId(): string {
  try {
    let id = localStorage.getItem(INSTALLATION_KEY);
    if (!id || !/^[A-Za-z0-9-]{16,64}$/.test(id)) {
      id = crypto.randomUUID();
      localStorage.setItem(INSTALLATION_KEY, id);
    }
    return id;
  } catch {
    return "";
  }
}

/** Sessions slide: once a token is a day old, any authenticated answer
 *  carries a fresh one. Without adopting it the app signed people out after
 *  thirty days however often they used it. */
function adoptRenewedToken(res: Response): void {
  const renewed = res.headers.get("X-Session-Token");
  const email = readStoredEmail();
  if (renewed && email) storeSession(renewed, email);
}

/** Signs in and returns the verified account. Throws an AccountRequestError
 *  whose key names the message to show. */
export async function login(email: string, password: string): Promise<AccountState> {
  if (!API_BASE_URL) throw new Error("Sign-in is not configured in this build.");
  const address = email.trim().toLowerCase();
  const res = await request(`/api/auth/login`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      email: address,
      password,
      product: PRODUCT,
      locale: mailLocale,
      deviceId: installationId(),
    }),
  });
  if (!res.ok) throw failure(res.status);
  const data = (await res.json()) as { token: string };
  storeSession(data.token, address);
  return fetchAccount();
}

/** Asks for a reset link. Answers the same whether or not the address has an
 *  account, like the server. */
export async function forgotPassword(email: string): Promise<void> {
  const res = await request(`/api/auth/forgot-password`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      email: email.trim().toLowerCase(),
      product: PRODUCT,
      locale: mailLocale,
    }),
  });
  if (!res.ok) throw failure(res.status);
}

export async function resendVerification(): Promise<void> {
  const token = readToken();
  if (!token) throw new AccountRequestError("errorCredentials");
  const res = await request(`/api/auth/resend-verification`, {
    method: "POST",
    headers: { "Content-Type": "application/json", Authorization: `Bearer ${token}` },
    body: JSON.stringify({ product: PRODUCT, locale: mailLocale }),
  });
  if (!res.ok) throw failure(res.status);
}

/** Needs the current password; this session continues with the token the
 *  server returns, every other one ends. */
export async function changePassword(currentPassword: string, newPassword: string): Promise<void> {
  const token = readToken();
  const email = readStoredEmail();
  if (!token || !email) throw new AccountRequestError("errorCredentials");
  const res = await request(`/api/auth/change-password`, {
    method: "POST",
    headers: { "Content-Type": "application/json", Authorization: `Bearer ${token}` },
    body: JSON.stringify({
      currentPassword,
      newPassword,
      product: PRODUCT,
      locale: mailLocale,
    }),
  });
  if (res.status === 401) throw new AccountRequestError("errorCurrentPassword");
  if (!res.ok) throw failure(res.status);
  const data = (await res.json()) as { token: string };
  storeSession(data.token, email);
}

/** Nothing changes until the link sent to the new address is confirmed. */
export async function changeEmail(password: string, newEmail: string): Promise<void> {
  const token = readToken();
  if (!token) throw new AccountRequestError("errorCredentials");
  const res = await request(`/api/auth/change-email`, {
    method: "POST",
    headers: { "Content-Type": "application/json", Authorization: `Bearer ${token}` },
    body: JSON.stringify({
      password,
      newEmail: newEmail.trim(),
      product: PRODUCT,
      locale: mailLocale,
    }),
  });
  if (res.status === 401) throw new AccountRequestError("errorCurrentPassword");
  if (!res.ok) throw failure(res.status);
}

export async function register(
  email: string,
  password: string,
  firstName: string,
  lastName: string,
  dateOfBirth: string,
): Promise<{ account: AccountState; verificationEmailSent: boolean }> {
  const res = await request(`/api/auth/register`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      email: email.trim().toLowerCase(),
      password,
      firstName,
      lastName,
      dateOfBirth,
      product: PRODUCT,
      locale: mailLocale,
      deviceId: installationId(),
    }),
  });
  if (!res.ok) throw failure(res.status);
  const data = (await res.json().catch(() => ({}))) as {
    token?: string;
    verificationEmailSent?: boolean;
  };
  if (!data.token) throw new AccountRequestError("errorServer");
  storeSession(data.token, email.trim().toLowerCase());
  return {
    account: await fetchAccount(),
    verificationEmailSent: data.verificationEmailSent === true,
  };
}

/**
 * Re-reads Pro status from the account. This — not any local file, not
 * installation detection — is the single source of truth for whether the
 * loyalty price and any Pro perk apply.
 */
export async function fetchAccount(): Promise<AccountState> {
  const token = readToken();
  const email = readStoredEmail();
  if (!token || !email) return { status: "anonymous" };
  if (!API_BASE_URL) return { status: "anonymous" };
  try {
    const res = await fetch(`${API_BASE_URL}/api/account`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    if (res.status === 401) {
      clearSession();
      return { status: "anonymous" };
    }
    if (!res.ok) {
      return { status: "error", email, message: "server" };
    }
    adoptRenewedToken(res);
    const data = (await res.json()) as {
      email: string;
      isPro: boolean;
      emailVerified: boolean;
      hasBilling?: boolean;
    };
    return {
      status: "signed-in",
      email: data.email,
      isPro: data.isPro,
      emailVerified: data.emailVerified,
      hasBilling: data.hasBilling === true,
    };
  } catch {
    // Offline: keep the user signed in locally rather than bouncing them to
    // anonymous on a transient network hiccup; Pro perks stay locked until a
    // successful check confirms them, which is the honest default.
    return { status: "error", email, message: "offline" };
  }
}

export function logout(): void {
  clearSession();
  void invoke("clear_license").catch(() => {});
}

export type UninstallerEntitlement = {
  active: boolean;
  plan: string | null;
  expiresAt?: string | null;
};

/** Offline or with the API unreachable, the signed licence cached on disk
 *  still decides: Pro keeps working for as long as that licence is fresh. */
async function cachedEntitlement(): Promise<UninstallerEntitlement | null> {
  try {
    return (await invoke<boolean>("license_status")) ? { active: true, plan: null } : null;
  } catch {
    return null;
  }
}

/** The per-product entitlement map; only the uninstaller's row matters here.
 *  Informational for the UI — enforcement lives server-side. */
export async function fetchUninstallerEntitlement(): Promise<UninstallerEntitlement | null> {
  const token = readToken();
  if (!token || !API_BASE_URL) return null;
  try {
    const license = await fetch(`${API_BASE_URL}/api/license?product=uninstaller`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    if (license.status === 401) return null;
    if (!license.ok) return await cachedEntitlement();
    const response: unknown = await license.json();
    if (readToken() !== token) return null;
    await invoke("save_license", { response });
    if (readToken() !== token) {
      await invoke("clear_license");
      return null;
    }
    const verified = await invoke<boolean>("license_status");
    const res = await fetch(`${API_BASE_URL}/api/entitlements`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    if (!res.ok) return verified ? { active: true, plan: null } : null;
    const data = (await res.json()) as {
      products?: {
        product: string;
        active: boolean;
        plan: string | null;
        expiresAt?: string | null;
      }[];
    };
    const row = data.products?.find((p) => p.product === "uninstaller");
    return row
      ? { active: row.active && verified, plan: row.plan, expiresAt: row.expiresAt ?? null }
      : { active: false, plan: null };
  } catch {
    return await cachedEntitlement();
  }
}

/** Trial and subscription state for this account (CONTRATTO.md §2.1), as
 *  sent: trial.ts parses it. Null on a backend that does not publish it yet. */
export async function fetchBillingStatus(): Promise<unknown> {
  const token = readToken();
  if (!token || !API_BASE_URL) return null;
  try {
    const res = await fetch(`${API_BASE_URL}/api/billing/status?product=uninstaller`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    if (!res.ok || readToken() !== token) return null;
    return (await res.json()) as unknown;
  } catch {
    return null;
  }
}

/** Public prices and the trial switch, as sent: the only source of a
 *  renewal price. */
export async function fetchPriceList(): Promise<unknown> {
  if (!API_BASE_URL) return null;
  try {
    const res = await fetch(`${API_BASE_URL}/api/billing/prices`, {
      cache: "no-store",
      signal: AbortSignal.timeout(8000),
    });
    return res.ok ? ((await res.json()) as unknown) : null;
  } catch {
    return null;
  }
}

/** Starts a Stripe Checkout for Uninstaller Pro (annual; the backend picks
 *  the loyalty price server-side when the account has PC Tweaker Pro) and
 *  returns the URL to open in the system browser. */
export async function startUninstallerCheckout(startTrial?: boolean): Promise<string> {
  const token = readToken();
  if (!token || !API_BASE_URL) throw new Error("Sign in first.");
  const res = await fetch(`${API_BASE_URL}/api/checkout`, {
    method: "POST",
    headers: { "Content-Type": "application/json", Authorization: `Bearer ${token}` },
    // start_trial only when a trial is on offer (trial button or "Subscribe
    // now"); the server still decides whether one applies.
    body: JSON.stringify(
      startTrial === undefined
        ? { product: "uninstaller", plan: "annual" }
        : { product: "uninstaller", plan: "annual", start_trial: startTrial },
    ),
  });
  const body = (await res.json().catch(() => ({}))) as { url?: string; error?: string };
  if (!res.ok || !body.url) throw new Error(body.error || `HTTP ${String(res.status)}`);
  return body.url;
}

/** Opens Stripe's hosted billing portal (cancel, card, invoices) for the
 *  signed-in account: the same POST /api/portal PC Tweaker uses. Returns the
 *  URL to open in the system browser. */
export async function startBillingPortal(): Promise<string> {
  const token = readToken();
  if (!token || !API_BASE_URL) throw new Error("Sign in first.");
  const res = await fetch(`${API_BASE_URL}/api/portal`, {
    method: "POST",
    headers: { Authorization: `Bearer ${token}` },
  });
  const body = (await res.json().catch(() => ({}))) as { url?: string; error?: string };
  if (!res.ok || !body.url) throw new Error(body.error || `HTTP ${String(res.status)}`);
  return body.url;
}
