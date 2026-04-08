export interface Wallet {
  id: string;
  name: string;
  owner_id: string;
  workspace_id: string | null;
  status: string;
  created_at: string;
  updated_at: string;
}

export interface Address {
  chain_id: string;
  address: string;
}

export interface Portfolio {
  chain_id: string;
  address: string;
  native_balance: string;
  assets: TokenAsset[];
}

export interface TokenAsset {
  chain_id: string;
  address: string;
  token_address: string;
  symbol: string;
  name: string;
  decimals: number;
  balance: string;
  balance_formatted: string;
}

export interface Tx {
  id: number;
  action: string;
  decision: string;
  tx_hash: string | null;
  created_at: string;
}

export interface ApiKey {
  id: string;
  name: string;
  permissions: string;
  expired: boolean;
}

export interface Policy {
  id: string;
  name: string;
  wallet_id: string | null;
  workspace_id: string | null;
  rules_json: string;
  status: string;
}
