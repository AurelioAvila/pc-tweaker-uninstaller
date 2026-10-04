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
  | { status: "signed-in"; email: string; isPro: boolean; emailVerified: boolean }
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

/** Signs in and returns the verified account. Throws with a message the UI
 *  can show verbatim. */
export async function login(email: string, password: string): Promise<AccountState> {
  if (!API_BASE_URL) throw new Error("Sign-in is not configured in this build.");
  const res = await fetch(`${API_BASE_URL}/api/auth/login`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ email, password }),
  });
  if (!res.ok) {
    const body = (await res.json().catch(() => ({}))) as { error?: string };
    throw new Error(body.error || `HTTP ${String(res.status)}`);
  }
  const data = (await res.json()) as { token: string };
  storeSession(data.token, email);
  return fetchAccount();
}

export async function register(
  email: string,
  password: string,
  firstName: string,
  lastName: string,
  dateOfBirth: string,
): Promise<{ account: AccountState; verificationEmailSent: boolean }> {
  const res = await fetch(`${API_BASE_URL}/api/auth/register`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ email, password, firstName, lastName, dateOfBirth }),
  });
  const data = (await res.json().catch(() => ({}))) as {
    token?: string;
    verificationEmailSent?: boolean;
    error?: string;
  };
  if (!res.ok || !data.token) throw new Error(data.error || `HTTP ${String(res.status)}`);
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
      return { status: "error", email, message: `HTTP ${String(res.status)}` };
    }
    const data = (await res.json()) as { email: string; isPro: boolean; emailVerified: boolean };
    return {
      status: "signed-in",
      email: data.email,
      isPro: data.isPro,
      emailVerified: data.emailVerified,
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

/** Starts a Stripe Checkout for Uninstaller Pro (annual; the backend picks
 *  the loyalty price server-side when the account has PC Tweaker Pro) and
 *  returns the URL to open in the system browser. */
export async function startUninstallerCheckout(): Promise<string> {
  const token = readToken();
  if (!token || !API_BASE_URL) throw new Error("Sign in first.");
  const res = await fetch(`${API_BASE_URL}/api/checkout`, {
    method: "POST",
    headers: { "Content-Type": "application/json", Authorization: `Bearer ${token}` },
    body: JSON.stringify({ product: "uninstaller", plan: "annual" }),
  });
  const body = (await res.json().catch(() => ({}))) as { url?: string; error?: string };
  if (!res.ok || !body.url) throw new Error(body.error || `HTTP ${String(res.status)}`);
  return body.url;
}
