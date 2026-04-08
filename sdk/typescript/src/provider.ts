import { GradienceClient } from "./client";
import type { TransactionRequest } from "./types";

type EventName = "connect" | "disconnect" | "chainChanged" | "accountsChanged";
type Listener = (...args: any[]) => void;

export interface EIP1193Provider {
  request(args: { method: string; params?: unknown[] }): Promise<unknown>;
  on(event: EventName, listener: Listener): void;
  removeListener(event: EventName, listener: Listener): void;
  enable?(): Promise<string[]>;
}

export interface GradienceProviderOptions {
  baseUrl: string;
  apiToken: string;
  walletId: string;
  chainId: string; // e.g. "0x2105" for Base
}

export class GradienceProvider implements EIP1193Provider {
  private client: GradienceClient;
  private walletId: string;
  private chainId: string;
  private listeners: Map<EventName, Set<Listener>> = new Map();

  constructor(opts: GradienceProviderOptions) {
    this.client = new GradienceClient(opts.baseUrl, { apiToken: opts.apiToken });
    this.walletId = opts.walletId;
    this.chainId = opts.chainId;
  }

  async request(args: { method: string; params?: unknown[] }): Promise<unknown> {
    const { method, params = [] } = args;
    switch (method) {
      case "eth_requestAccounts":
      case "eth_accounts": {
        const addrs = await this.client.listWalletAddresses(this.walletId);
        const evm = addrs
          .filter((a: any) => a.chain_id.startsWith("eip155:"))
          .map((a: any) => a.address);
        return evm.length ? evm : [];
      }
      case "eth_chainId":
        return this.chainId;
      case "eth_sendTransaction": {
        const tx = (params[0] as Record<string, any>) || {};
        const payload: TransactionRequest = {
          to: tx.to,
          value: tx.value || "0",
          data: tx.data || "0x",
          chainId: this.chainId,
        };
        const signed = await this.client.signTransaction(this.walletId, payload);
        return signed.signed_tx;
      }
      case "personal_sign":
      case "eth_sign": {
        // Not directly supported by current REST API; caller can fall back to API key + MCP.
        throw new Error("Method not implemented by GradienceProvider: " + method);
      }
      default:
        throw new Error("Method not supported: " + method);
    }
  }

  on(event: EventName, listener: Listener): void {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set());
    }
    this.listeners.get(event)!.add(listener);
  }

  removeListener(event: EventName, listener: Listener): void {
    this.listeners.get(event)?.delete(listener);
  }

  emit(event: EventName, ...args: any[]): void {
    this.listeners.get(event)?.forEach((fn) => fn(...args));
  }

  async enable(): Promise<string[]> {
    const accounts = await this.request({ method: "eth_requestAccounts" });
    this.emit("connect", { chainId: this.chainId });
    return accounts as string[];
  }

  disconnect(): void {
    this.emit("disconnect");
  }

  setChainId(chainId: string): void {
    this.chainId = chainId;
    this.emit("chainChanged", chainId);
  }
}
