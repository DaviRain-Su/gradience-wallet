import { create, get } from "@github/webauthn-json";
import { apiPost } from "./api";

export async function registerPasskey(username: string, passphrase: string) {
  const startRes = await apiPost("/api/auth/passkey/register/start", { username });
  const { challenge } = await startRes.json();

  const credential = await create({ publicKey: challenge });

  const finishRes = await apiPost("/api/auth/passkey/register/finish", {
    username,
    credential,
    passphrase,
  });
  const { token } = await finishRes.json();
  localStorage.setItem("gradience_token", token);
  return token;
}

export async function loginPasskey(username: string) {
  const startRes = await apiPost("/api/auth/passkey/login/start", { username });
  const { challenge } = await startRes.json();

  const credential = await get({ publicKey: challenge });

  const finishRes = await apiPost("/api/auth/passkey/login/finish", {
    username,
    credential,
  });
  const { token } = await finishRes.json();
  localStorage.setItem("gradience_token", token);
  return token;
}

export async function unlockVault(passphrase: string) {
  const token = localStorage.getItem("gradience_token");
  if (!token) throw new Error("No session");
  const res = await fetch(`${process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080"}/api/auth/unlock`, {
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
