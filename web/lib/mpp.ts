import { apiDelete, apiGet, apiPost } from "./api";

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
  const res = await apiGet(`/api/ai/proxy-keys?wallet_id=${encodeURIComponent(walletId)}`);
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`Failed to list keys: ${res.status} ${text}`);
  }
  return res.json() as Promise<AiProxyKey[]>;
}

export async function createAiProxyKey(req: CreateAiProxyKeyReq): Promise<CreateAiProxyKeyResp> {
  const res = await apiPost("/api/ai/proxy-keys", req);
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`Failed to create key: ${res.status} ${text}`);
  }
  return res.json() as Promise<CreateAiProxyKeyResp>;
}

export async function deleteAiProxyKey(keyId: string): Promise<void> {
  const res = await apiDelete(`/api/ai/proxy-keys/${encodeURIComponent(keyId)}`);
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
