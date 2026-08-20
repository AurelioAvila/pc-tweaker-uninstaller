/**
 * Suite account: login against the SAME backend PC Tweaker uses, so a
 * PC Tweaker Pro subscription is what actually gates loyalty pricing and any
 * Pro perk in this app — never local detection of PC Tweaker being
 * installed. Installed-on-this-PC is a hint shown as a badge; it proves
 * nothing about the account, and nothing here treats it as if it did.
 *
 * Registration deliberately does NOT happen in this app: the site
 * (pctweaker.app) owns account creation, and this app only signs in with an
 * existing suite account.
 */

export const API_BASE_URL: string = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? "";

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
}
