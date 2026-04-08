import { apiGetRawBase } from "./api";

function getAiProxyBase(): string {
  if (typeof window === "undefined") {
    return process.env.NEXT_PUBLIC_AI_PROXY_URL?.trim() || apiGetRawBase();
  }
  const env = process.env.NEXT_PUBLIC_AI_PROXY_URL?.trim();
  if (env) return env;
  if (window.location.hostname.endsWith("gradiences.xyz")) {
    return window.location.origin.replace(/\/+$/, "");
  }
  return apiGetRawBase();
}

async function proxyPost(path: string, body: unknown) {
  const base = getAiProxyBase();
  const token = typeof window !== "undefined" ? localStorage.getItem("gradience_token") : null;
  const res = await fetch(`${base}${path}`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    },
    body: JSON.stringify(body),
  });
  if (!res.ok) {
    const text = await res.text().catch(() => "Unknown error");
    throw new Error(text || `HTTP ${res.status}`);
  }
  return res;
}

async function proxyGet(path: string) {
  const base = getAiProxyBase();
  const token = typeof window !== "undefined" ? localStorage.getItem("gradience_token") : null;
  const res = await fetch(`${base}${path}`, {
    method: "GET",
    headers: {
      "Content-Type": "application/json",
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    },
  });
  if (!res.ok) {
    const text = await res.text().catch(() => "Unknown error");
    throw new Error(text || `HTTP ${res.status}`);
  }
  return res;
}

async function proxyDelete(path: string) {
  const base = getAiProxyBase();
  const token = typeof window !== "undefined" ? localStorage.getItem("gradience_token") : null;
  const res = await fetch(`${base}${path}`, {
    method: "DELETE",
    headers: {
      "Content-Type": "application/json",
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    },
  });
  if (!res.ok) {
    const text = await res.text().catch(() => "Unknown error");
    throw new Error(text || `HTTP ${res.status}`);
  }
  return res;
}

export interface AiProxyKey {
  id: string;
  name: string;
  permissions: string;
  expires_at?: string;
  created_at: string;
}

export interface CreateAiProxyKeyReq {
  wallet_id: string;
  name: string;
}

export interface CreateAiProxyKeyResp {
  id: string;
  name: string;
  raw_token: string;
  permissions: string;
  expires_at: string;
}

export async function listAiProxyKeys(walletId: string): Promise<AiProxyKey[]> {
  const res = await proxyGet(`/api/ai/proxy-keys?wallet_id=${encodeURIComponent(walletId)}`);
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`Failed to list keys: ${res.status} ${text}`);
  }
  return res.json() as Promise<AiProxyKey[]>;
}

export async function createAiProxyKey(req: CreateAiProxyKeyReq): Promise<CreateAiProxyKeyResp> {
  const res = await proxyPost("/api/ai/proxy-keys", req);
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`Failed to create key: ${res.status} ${text}`);
  }
  return res.json() as Promise<CreateAiProxyKeyResp>;
}

export async function deleteAiProxyKey(keyId: string): Promise<void> {
  const res = await proxyDelete(`/api/ai/proxy-keys/${encodeURIComponent(keyId)}`);
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`Failed to delete key: ${res.status} ${text}`);
  }
}

// Future: direct-browser MPP client via mppx SDK
// import { Mppx } from "mppx";
// export function createDirectMppClient(credentials: string[]) {
//   return Mppx.create({ credentials });
// }
