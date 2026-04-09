export { GradienceClient, GradienceError } from "./client";
export type {
  GradienceClientOptions,
  Wallet,
  Balance,
  SwapQuoteParams,
  SwapQuoteResult,
  AiGenerateParams,
  AiGenerateResult,
  TransactionRequest,
  SignResult,
  Policy,
  MppChain,
  MppChargeParams,
  MppChargeResult,
} from "./types";
export { MPP_SUPPORTED_CHAINS } from "./types";

export { GradienceProvider, type GradienceProviderOptions, type EIP1193Provider } from "./provider";

export {
  useWallets,
  useWalletBalance,
  useCreateWallet,
  usePolicies,
  useCreatePolicy,
  useSwapQuote,
  useAiGenerate,
  type GradienceReactOptions,
} from "./react/hooks";

export { GradienceMcpClient, type McpCallOptions } from "./mcp";
