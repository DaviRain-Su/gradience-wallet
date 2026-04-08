import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";
import type { Transport } from "@modelcontextprotocol/sdk/shared/transport.js";

export interface McpCallOptions {
  walletId: string;
  apiToken?: string;
}

export class GradienceMcpClient {
  private client: Client;
  private transport: Transport;
  private connected = false;

  constructor(transport: Transport) {
    this.transport = transport;
    this.client = new Client({ name: "gradience-sdk", version: "0.1.0" });
  }

  static async fromStdio(command: string, args: string[] = []): Promise<GradienceMcpClient> {
    const transport = new StdioClientTransport({ command, args });
    const instance = new GradienceMcpClient(transport);
    await instance.connect();
    return instance;
  }

  async connect(): Promise<void> {
    if (this.connected) return;
    await this.client.connect(this.transport);
    this.connected = true;
  }

  async disconnect(): Promise<void> {
    if (!this.connected) return;
    await this.client.close();
    this.connected = false;
  }

  async listTools(): Promise<any> {
    this.ensureConnected();
    return this.client.listTools();
  }

  async getBalance(walletId: string, chain?: string): Promise<any> {
    return this.callTool("get_balance", { wallet_id: walletId, chain });
  }

  async signTransaction(walletId: string, transaction: Record<string, unknown>): Promise<any> {
    return this.callTool("sign_transaction", { wallet_id: walletId, transaction });
  }

  async signMessage(walletId: string, message: string): Promise<any> {
    return this.callTool("sign_message", { wallet_id: walletId, message });
  }

  async signAndSend(walletId: string, transaction: Record<string, unknown>): Promise<any> {
    return this.callTool("sign_and_send", { wallet_id: walletId, transaction });
  }

  async swap(walletId: string, params: Record<string, unknown>): Promise<any> {
    return this.callTool("swap", { wallet_id: walletId, ...params });
  }

  async pay(walletId: string, recipient: string, amount: string, chain?: string, token?: string): Promise<any> {
    return this.callTool("pay", { wallet_id: walletId, recipient, amount, chain, token });
  }

  async llmGenerate(walletId: string, model: string, prompt: string): Promise<any> {
    return this.callTool("llm_generate", { wallet_id: walletId, model, prompt });
  }

  async aiModels(): Promise<any> {
    return this.callTool("ai_models", {});
  }

  async verifyApiKey(apiKey: string): Promise<any> {
    return this.callTool("verify_api_key", { api_key: apiKey });
  }

  async callTool(name: string, args: Record<string, unknown>): Promise<any> {
    this.ensureConnected();
    const result = await this.client.callTool({ name, arguments: args });
    return result;
  }

  private ensureConnected(): void {
    if (!this.connected) {
      throw new Error("MCP client not connected. Call connect() first.");
    }
  }
}
