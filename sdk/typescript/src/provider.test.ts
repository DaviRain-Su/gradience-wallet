import { GradienceProvider } from "./provider";

describe("GradienceProvider", () => {
  const fetchMock = jest.fn();

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

  function createProvider(): GradienceProvider {
    return new GradienceProvider({
      baseUrl: "http://localhost:8080",
      apiToken: "token",
      walletId: "w1",
      chainId: "0x2105",
    });
  }

  it("returns accounts for eth_requestAccounts", async () => {
    mockResponse(200, [{ chain_id: "eip155:8453", address: "0xabc", derivation_path: "m/44'/60'/0'/0/0" }]);
    const provider = createProvider();
    const accounts = await provider.request({ method: "eth_requestAccounts" });
    expect(accounts).toEqual(["0xabc"]);
  });

  it("returns chainId for eth_chainId", async () => {
    const provider = createProvider();
    const chainId = await provider.request({ method: "eth_chainId" });
    expect(chainId).toBe("0x2105");
  });

  it("signs transaction via eth_sendTransaction", async () => {
    mockResponse(200, { signed_tx: "0xdeadbeef" });
    const provider = createProvider();
    const signed = await provider.request({
      method: "eth_sendTransaction",
      params: [{ to: "0xto", value: "1000", data: "0x" }],
    });
    expect(signed).toBe("0xdeadbeef");
  });

  it("emits connect on enable", async () => {
    mockResponse(200, [{ chain_id: "eip155:8453", address: "0xabc", derivation_path: "m/44'/60'/0'/0/0" }]);
    const provider = createProvider();
    const connectHandler = jest.fn();
    provider.on("connect", connectHandler);
    await provider.enable();
    expect(connectHandler).toHaveBeenCalledWith({ chainId: "0x2105" });
  });

  it("throws for unsupported methods", async () => {
    const provider = createProvider();
    await expect(provider.request({ method: "eth_sign" })).rejects.toThrow("not implemented");
  });
});
