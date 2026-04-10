export interface GradienceClientOptions {
  apiToken?: string;
}

export interface Wallet {
  id: string;
  name: string;
  status: string;
  created_at: string;
}

export interface Balance {
  chain_id: string;
  token_address: string;
  balance: string;
  decimals: number;
}

export interface SwapQuoteParams {
  fromToken: string;
  toToken: string;
  amount: string;
  chain?: string;
}

export interface SwapQuoteResult {
  from_token: string;
  to_token: string;
  from_amount: string;
  to_amount: string;
  chain: string;
}

export interface AiGenerateParams {
  walletId: string;
  provider?: string;
  model: string;
  prompt: string;
}

export interface AiGenerateResult {
  text: string;
  cost: string;
}

export interface TransactionRequest {
  to: string;
  value: string;
  data?: string;
  chainId?: string;
}

export interface SignResult {
  signed_tx: string;
  tx_hash?: string;
}

export interface Policy {
  id: string;
  name: string;
  wallet_id: string | null;
  workspace_id: string | null;
  rules_json: string;
  status: string;
  created_at: string;
}

export type MppChain =
  | "tempo"
  | "base"
  | "bsc"
  | "conflux"
  | "xlayer"
  | "arbitrum"
  | "polygon"
  | "optimism"
  | "solana"
  | "ton";

export const MPP_SUPPORTED_CHAINS: MppChain[] = [
  "tempo",
  "base",
  "bsc",
  "conflux",
  "xlayer",
  "arbitrum",
  "polygon",
  "optimism",
  "solana",
  "ton",
];

export interface MppChargeParams {
  walletId: string;
  provider: string;
  model: string;
  prompt: string;
  preferredChain?: MppChain;
}

export interface MppChargeResult {
  provider_status: number;
  data: unknown;
  chain_used?: string;
}
