import { renderHook, waitFor } from "@testing-library/react";
import {
  useWallets,
  useWalletBalance,
  useCreateWallet,
  usePolicies,
  useSwapQuote,
} from "./hooks";

const fetchMock = jest.fn();

describe("React Hooks", () => {
  beforeEach(() => {
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

  const opts = { baseUrl: "http://localhost:8080", apiToken: "token" };

  it("useWallets fetches on mount", async () => {
    mockResponse(200, [{ id: "w1", name: "demo", status: "active", created_at: "2026-01-01T00:00:00Z" }]);
    const { result } = renderHook(() => useWallets(opts));
    await waitFor(() => expect(result.current.wallets).not.toBeNull());
    expect(result.current.wallets![0].id).toBe("w1");
    expect(result.current.loading).toBe(false);
  });

  it("useWalletBalance fetches when walletId provided", async () => {
    mockResponse(200, [{ chain_id: "eip155:8453", token_address: "0x0", balance: "1000", decimals: 6 }]);
    const { result } = renderHook(() => useWalletBalance(opts, "w1"));
    await waitFor(() => expect(result.current.balance).not.toBeNull());
    expect(result.current.balance![0].balance).toBe("1000");
  });

  it("useCreateWallet creates a wallet", async () => {
    mockResponse(200, { id: "w2", name: "new", status: "active", created_at: "2026-01-01T00:00:00Z" });
    const { result } = renderHook(() => useCreateWallet(opts));
    const wallet = await result.current.create("new");
    expect(wallet.id).toBe("w2");
  });

  it("usePolicies fetches policies", async () => {
    mockResponse(200, [{ id: "p1", name: "policy", wallet_id: "w1", workspace_id: null, rules_json: "{}", status: "active", created_at: "2026-01-01T00:00:00Z" }]);
    const { result } = renderHook(() => usePolicies(opts, "w1"));
    await waitFor(() => expect(result.current.policies).not.toBeNull());
    expect(result.current.policies![0].id).toBe("p1");
  });

  it("useSwapQuote fetches quote on demand", async () => {
    mockResponse(200, { from_token: "0xA", to_token: "0xB", from_amount: "1000", to_amount: "2000", chain: "base" });
    const { result } = renderHook(() => useSwapQuote(opts));
    const quote = await result.current.fetchQuote("w1", { fromToken: "0xA", toToken: "0xB", amount: "1000" });
    expect(quote.to_amount).toBe("2000");
  });
});
