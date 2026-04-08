import { GradienceClient, GradienceError } from "./client";

describe("GradienceClient", () => {
  let client: GradienceClient;
  const fetchMock = jest.fn();

  beforeEach(() => {
    client = new GradienceClient("http://localhost:8080", { apiToken: "test-token" });
    global.fetch = fetchMock;
    fetchMock.mockClear();
  });

  afterEach(() => {
    jest.restoreAllMocks();
  });

  function mockResponse(status: number, body: unknown) {
    fetchMock.mockResolvedValueOnce({
      ok: status < 400,
      status,
      json: async () => body,
      text: async () => JSON.stringify(body),
    } as Response);
  }

  it("creates a wallet", async () => {
    mockResponse(201, { id: "w1", name: "demo", status: "active", created_at: "2026-01-01T00:00:00Z" });
    const wallet = await client.createWallet("demo");
    expect(wallet.id).toBe("w1");
    expect(wallet.name).toBe("demo");
    expect(fetchMock).toHaveBeenCalledWith(
      "http://localhost:8080/api/wallets",
      expect.objectContaining({ method: "POST", body: JSON.stringify({ name: "demo" }) })
    );
  });

  it("lists wallets", async () => {
    mockResponse(200, [{ id: "w1", name: "demo", status: "active", created_at: "2026-01-01T00:00:00Z" }]);
    const wallets = await client.listWallets();
    expect(wallets).toHaveLength(1);
    expect(wallets[0].id).toBe("w1");
  });

  it("gets balance", async () => {
    mockResponse(200, [{ chain_id: "eip155:8453", token_address: "0x0", balance: "1000", decimals: 6 }]);
    const balances = await client.getBalance("w1");
    expect(balances[0].balance).toBe("1000");
  });

  it("funds a wallet", async () => {
    mockResponse(200, { txHash: "0xabc" });
    const res = await client.fundWallet("w1", "0xto", "1", "base");
    expect(res.txHash).toBe("0xabc");
  });

  it("fetches swap quote", async () => {
    mockResponse(200, {
      from_token: "0xA",
      to_token: "0xB",
      from_amount: "1000",
      to_amount: "2000",
      chain: "base",
    });
    const quote = await client.swapQuote("w1", {
      fromToken: "0xA",
      toToken: "0xB",
      amount: "1000",
      chain: "base",
    });
    expect(quote.to_amount).toBe("2000");
  });

  it("generates ai text", async () => {
    mockResponse(200, { text: "hello", cost: "0.001" });
    const res = await client.aiGenerate({ walletId: "w1", model: "claude", prompt: "hi" });
    expect(res.text).toBe("hello");
  });

  it("throws GradienceError on non-ok response", async () => {
    mockResponse(400, { error: "bad request" });
    try {
      await client.createWallet("");
    } catch (e: any) {
      expect(e).toBeInstanceOf(GradienceError);
      expect(e.message).toBe("bad request");
      expect(e.statusCode).toBe(400);
    }
  });

  it("exports audit logs", async () => {
    mockResponse(200, [{ id: "a1", action: "sign" }]);
    const logs = await client.exportAudit("w1", "json");
    expect(Array.isArray(logs)).toBe(true);
  });
});
