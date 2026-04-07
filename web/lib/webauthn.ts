import { create, get } from "@github/webauthn-json";
import { apiPost, apiGetRawBase } from "./api";

export async function registerPasskey(username: string, passphrase: string, email?: string) {
  const startRes = await apiPost("/api/auth/passkey/register/start", { username });
  const { challenge } = await startRes.json();

  const credential = await create(challenge);

  const finishRes = await apiPost("/api/auth/passkey/register/finish", {
    username,
    credential,
    passphrase,
    ...(email ? { email } : {}),
  });
  const { token } = await finishRes.json();
  localStorage.setItem("gradience_token", token);
  return token;
}

export async function loginPasskey(username: string) {
  const startRes = await apiPost("/api/auth/passkey/login/start", { username });
  const { challenge } = await startRes.json();

  const credential = await get(challenge);

  const finishRes = await apiPost("/api/auth/passkey/login/finish", {
    username,
    credential,
  });
  const { token } = await finishRes.json();
  localStorage.setItem("gradience_token", token);
  return token;
}

export async function registerPasskeyForRecovery(username: string, recoveryToken: string) {
  const startRes = await apiPost("/api/auth/passkey/register/start", { username });
  const { challenge } = await startRes.json();

  const credential = await create(challenge);

  const finishRes = await apiPost("/api/auth/recover/register", {
    recovery_token: recoveryToken,
    credential,
  });
  const { token } = await finishRes.json();
  localStorage.setItem("gradience_token", token);
  return token;
}

export async function unlockVault(passphrase: string) {
  const token = localStorage.getItem("gradience_token");
  if (!token) throw new Error("No session");
  const base = apiGetRawBase();
  const res = await fetch(`${base}/api/auth/unlock`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${token}`,
    },
    body: JSON.stringify({ passphrase }),
  });
  if (!res.ok) throw new Error("Unlock failed");
}

export function getToken(): string | null {
  return localStorage.getItem("gradience_token");
}

export function authHeaders(): Record<string, string> {
  const token = getToken();
  return token ? { Authorization: `Bearer ${token}` } : {};
}
