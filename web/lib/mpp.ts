import { apiPost } from "./api";

export type MppProvider =
  | "anthropic"
  | "openai"
  | "openrouter"
  | "gemini"
  | "groq"
  | "mistral"
  | "deepseek";

export interface MppGenerateReq {
  wallet_id: string;
  provider: MppProvider;
  model: string;
  prompt: string;
}

export interface MppGenerateResp {
  provider_status: number;
  data: unknown;
}

export async function mppGenerate(req: MppGenerateReq): Promise<MppGenerateResp> {
  const res = await apiPost("/api/ai/mpp-generate", req);
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`MPP generate failed: ${res.status} ${text}`);
  }
  return res.json() as Promise<MppGenerateResp>;
}

// Future: direct-browser MPP client via mppx SDK
// import { Mppx } from "mppx";
// export function createDirectMppClient(credentials: string[]) {
//   return Mppx.create({ credentials });
// }
