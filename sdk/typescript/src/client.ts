import type {
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
  MppChargeParams,
  MppChargeResult,
} from "./types";

export class GradienceError extends Error {
  statusCode?: number;
  body?: unknown;

  constructor(message: string, statusCode?: number, body?: unknown) {
    super(message);
    this.statusCode = statusCode;
    this.body = body;
  }
}

export class GradienceClient {
  private baseUrl: string;
  private apiToken?: string;

  constructor(baseUrl: string, options: GradienceClientOptions = {}) {
    this.baseUrl = baseUrl.replace(/\/$/, "");
    this.apiToken = options.apiToken;
  }

  private headers(): Record<string, string> {
    const h: Record<string, string> = {
      "Content-Type": "application/json",
    };
    if (this.apiToken) {
      h["Authorization"] = `Bearer ${this.apiToken}`;
    }
    return h;
  }

  private async request<T>(
    method: string,
    path: string,
    opts?: { body?: unknown; query?: Record<string, string | undefined> }
  ): Promise<T> {
    let url = `${this.baseUrl}${path}`;
    if (opts?.query) {
      const search = new URLSearchParams();
      for (const [k, v] of Object.entries(opts.query)) {
        if (v !== undefined) search.set(k, v);
      }
      const q = search.toString();
      if (q) url += `?${q}`;
    }

    const resp = await fetch(url, {
      method,
      headers: this.headers(),
      body: opts?.body ? JSON.stringify(opts.body) : undefined,
    });

    let data: unknown;
    try {
      data = await resp.json();
    } catch {
      data = await resp.text();
    }

    if (!resp.ok) {
      const msg =
        typeof data === "object" && data !== null && "error" in data
          ? String((data as { error?: string }).error)
          : String(data);
      throw new GradienceError(msg, resp.status, data);
    }

    return data as T;
  }

  async createWallet(name: string): Promise<Wallet> {
    return this.request<Wallet>("POST", "/api/wallets", { body: { name } });
  }

  async listWallets(): Promise<Wallet[]> {
    return this.request<Wallet[]>("GET", "/api/wallets");
  }

  async getBalance(walletId: string): Promise<Balance[]> {
    return this.request<Balance[]>("GET", `/api/wallets/${walletId}/balance`);
  }

  async fundWallet(
    walletId: string,
    to: string,
    amount: string,
    chain: string = "base"
  ): Promise<{ txHash: string }> {
    return this.request<{ txHash: string }>("POST", `/api/wallets/${walletId}/fund`, {
      body: { to, amount, chain },
    });
  }

  async signTransaction(walletId: string, transaction: TransactionRequest): Promise<SignResult> {
    return this.request<SignResult>("POST", `/api/wallets/${walletId}/sign`, {
      body: { transaction },
    });
  }

  async listTransactions(walletId: string): Promise<unknown[]> {
    return this.request<unknown[]>("GET", `/api/wallets/${walletId}/transactions`);
  }

  async swapQuote(walletId: string, params: SwapQuoteParams): Promise<SwapQuoteResult> {
    return this.request<SwapQuoteResult>("GET", "/api/swap/quote", {
      query: {
        wallet_id: walletId,
        from_token: params.fromToken,
        to_token: params.toToken,
        amount: params.amount,
        chain: params.chain ?? "base",
      },
    });
  }

  async getAiBalance(walletId: string): Promise<{ token: string; balance: string }> {
    return this.request<{ token: string; balance: string }>(
      "GET",
      `/api/ai/balance/${walletId}`
    );
  }

  async aiGenerate(params: AiGenerateParams): Promise<AiGenerateResult> {
    return this.request<AiGenerateResult>("POST", "/api/ai/generate", {
      body: { wallet_id: params.walletId, model: params.model, prompt: params.prompt },
    });
  }

  async listPolicies(walletId: string): Promise<Policy[]> {
    return this.request<Policy[]>("GET", `/api/wallets/${walletId}/policies`);
  }

  async createPolicy(walletId: string, content: string): Promise<{ policy_id: string }> {
    return this.request<{ policy_id: string }>("POST", `/api/wallets/${walletId}/policies`, {
      body: { content },
    });
  }

  async createWorkspacePolicy(workspaceId: string, content: string): Promise<{ policy_id: string }> {
    return this.request<{ policy_id: string }>(
      "POST",
      `/api/workspaces/${workspaceId}/policies`,
      { body: { content } }
    );
  }

  async exportAudit(walletId: string, format: "json" | "csv" = "json"): Promise<unknown> {
    return this.request<unknown>("GET", `/api/wallets/${walletId}/audit/export`, {
      query: { format },
    });
  }

  async createWorkspace(name: string): Promise<{ workspace_id: string }> {
    return this.request<{ workspace_id: string }>("POST", "/api/workspaces", {
      body: { name },
    });
  }

  async listWorkspaces(): Promise<{ id: string; name: string }[]> {
    return this.request<{ id: string; name: string }[]>("GET", "/api/workspaces");
  }

  async listWalletAddresses(walletId: string): Promise<{ address: string; chain_id: string; derivation_path: string }[]> {
    return this.request<{ address: string; chain_id: string; derivation_path: string }[]>(
      "GET",
      `/api/wallets/${walletId}/addresses`
    );
  }

  async mppGenerate(params: MppChargeParams): Promise<MppChargeResult> {
    return this.request<MppChargeResult>("POST", "/api/ai/mpp-generate", {
      body: {
        wallet_id: params.walletId,
        provider: params.provider,
        model: params.model,
        prompt: params.prompt,
        preferred_chain: params.preferredChain,
      },
    });
  }
}
