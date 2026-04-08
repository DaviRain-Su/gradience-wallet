"use client";

import { useEffect, useState } from "react";
import { apiGet, apiPost, setApiBase } from "@/lib/api";
import { formatChainName } from "@/lib/chains";

interface Wallet {
  id: string;
  name: string;
  owner_id: string;
  workspace_id: string | null;
  status: string;
  created_at: string;
  updated_at: string;
}

interface Address {
  chain_id: string;
  address: string;
}

interface Portfolio {
  chain_id: string;
  address: string;
  native_balance: string;
  assets: TokenAsset[];
}

interface TokenAsset {
  chain_id: string;
  address: string;
  token_address: string;
  symbol: string;
  name: string;
  decimals: number;
  balance: string;
  balance_formatted: string;
}

interface Tx {
  id: number;
  action: string;
  decision: string;
  tx_hash: string | null;
  created_at: string;
}

interface ApiKey {
  id: string;
  name: string;
  permissions: string;
  expired: boolean;
}

interface Policy {
  id: string;
  name: string;
  wallet_id: string | null;
  workspace_id: string | null;
  rules_json: string;
  status: string;
}

interface PendingApproval {
  status: string;
}

export default function Dashboard() {
  const [wallets, setWallets] = useState<Wallet[]>([]);
  const [name, setName] = useState("");
  const [loading, setLoading] = useState(false);
  const [msg, setMsg] = useState("");
  const [apiBase, setApiBaseState] = useState("");
  const [showApiConfig, setShowApiConfig] = useState(false);
  const [pendingApprovals, setPendingApprovals] = useState(0);
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
    if (typeof window !== "undefined") {
      const saved = localStorage.getItem("gradience_api_base") || "http://localhost:8080";
      setApiBaseState(saved);
      setShowApiConfig(window.location.protocol === "https:" && saved.startsWith("http:"));
    }
  }, []);

  async function fetchWallets() {
    try {
      const res = await apiGet("/api/wallets");
      const data = await res.json();
      setWallets(data);
    } catch (e: unknown) {
      setMsg(`Failed to load wallets: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  async function fetchPendingApprovals() {
    try {
      const res = await apiGet("/api/policy-approvals");
      const data: PendingApproval[] = await res.json();
      setPendingApprovals(data.filter((a) => a.status === "pending").length);
    } catch {
      // ignore
    }
  }

  useEffect(() => {
    fetchWallets();
    fetchPendingApprovals();
  }, []);

  async function handleCreate() {
    if (!name.trim()) return;
    setLoading(true);
    try {
      await apiPost("/api/wallets", { name });
      setName("");
      await fetchWallets();
    } catch (e: unknown) {
      setMsg(`Create failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="min-h-screen p-8 max-w-4xl mx-auto" style={{ backgroundColor: "var(--background)", color: "var(--foreground)" }}>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold">Wallet Dashboard</h1>
        <div className="flex items-center gap-3">
          <a
            href="/policies"
            className="text-sm px-3 py-1.5 rounded"
            style={{ backgroundColor: "var(--muted)", color: "var(--foreground)", border: "1px solid var(--border)" }}
          >
            Policies
          </a>
          <a
            href="/approvals"
            className="text-sm px-3 py-1.5 rounded relative"
            style={{ backgroundColor: "var(--muted)", color: "var(--foreground)", border: "1px solid var(--border)" }}
          >
            Approvals
            {pendingApprovals > 0 && (
              <span
                className="absolute -top-1.5 -right-1.5 text-xs font-bold px-1.5 py-0.5 rounded-full"
                style={{ backgroundColor: "#F59E0B", color: "#fff" }}
              >
                {pendingApprovals}
              </span>
            )}
          </a>
        </div>
      </div>

      <div className="flex gap-2 mb-6">
        <input
          className="border rounded px-3 py-2 flex-1"
          style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
          placeholder="Wallet name"
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
        <button
          onClick={handleCreate}
          disabled={loading}
          className="px-4 py-2 rounded disabled:opacity-50"
          style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}
        >
          {loading ? "Creating..." : "Create Wallet"}
        </button>
      </div>

      {mounted && showApiConfig && (
        <div className="mb-4 border rounded p-3" style={{ backgroundColor: "#FEF3C7", borderColor: "#FDE68A" }}>
          <p className="text-sm text-yellow-900 mb-2">
            You are on HTTPS but your local API is HTTP. Please enter your local API tunnel URL (e.g. ngrok HTTPS) or switch to local dev mode.
          </p>
          <div className="flex gap-2">
            <input
              className="border rounded px-2 py-1 flex-1 text-sm"
              style={{ backgroundColor: "var(--card)", borderColor: "var(--border)" }}
              placeholder="https://your-ngrok-url.ngrok-free.app"
              value={apiBase}
              onChange={(e) => setApiBaseState(e.target.value)}
            />
            <button
              onClick={() => { setApiBase(apiBase); setShowApiConfig(false); setMsg("API base updated. Refreshing..."); window.location.reload(); }}
              className="px-3 py-1 rounded text-sm"
              style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}
            >
              Save
            </button>
          </div>
        </div>
      )}

      {msg && <p className="text-sm mb-4" style={{ color: "#B45309" }}>{msg}</p>}

      <div className="space-y-6">
        {wallets.length === 0 && <p style={{ color: "var(--muted-foreground)" }}>No wallets yet.</p>}
        {wallets.map((w) => (
          <WalletCard key={w.id} wallet={w} />
        ))}
      </div>
    </div>
  );
}

function WalletCard({ wallet }: { wallet: Wallet }) {
  const [portfolio, setPortfolio] = useState<Portfolio[]>([]);
  const [addresses, setAddresses] = useState<Address[]>([]);
  const [txs, setTxs] = useState<Tx[]>([]);
  const [keys, setKeys] = useState<ApiKey[]>([]);
  const [policies, setPolicies] = useState<Policy[]>([]);
  const [newApiKeyToken, setNewApiKeyToken] = useState<string | null>(null);
  const [showFund, setShowFund] = useState(false);
  const [showKey, setShowKey] = useState(false);
  const [showSwap, setShowSwap] = useState(false);
  const [fundTo, setFundTo] = useState("");
  const [fundAmount, setFundAmount] = useState("0.001");
  const [keyName, setKeyName] = useState("");
  const [swapFrom, setSwapFrom] = useState("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913");
  const [swapTo, setSwapTo] = useState("0x4200000000000000000000000000000000000006");
  const [swapAmount, setSwapAmount] = useState("1");
  const [msg, setMsg] = useState("");
  const [fundLoading, setFundLoading] = useState(false);
  const [keyLoading, setKeyLoading] = useState(false);
  const [anchorLoading, setAnchorLoading] = useState(false);
  const [swapLoading, setSwapLoading] = useState(false);

  const [fundChain, setFundChain] = useState("base");
  const [swapChain, setSwapChain] = useState("base");

  useEffect(() => {
    apiGet(`/api/wallets/${wallet.id}/portfolio`).then((r) => r.json().then(setPortfolio)).catch((e) => console.error("portfolio fetch failed", e));
    apiGet(`/api/wallets/${wallet.id}/addresses`).then((r) => r.json().then(setAddresses)).catch((e) => {
      console.error("addresses fetch failed", e);
      setMsg(`Addresses load failed: ${e instanceof Error ? e.message : String(e)}`);
    });
    apiGet(`/api/wallets/${wallet.id}/transactions`).then((r) => r.json().then(setTxs)).catch((e) => console.error("txs fetch failed", e));
    apiGet(`/api/wallets/${wallet.id}/api-keys`).then((r) => r.json().then(setKeys)).catch((e) => console.error("keys fetch failed", e));
    apiGet(`/api/wallets/${wallet.id}/policies`).then((r) => r.json().then(setPolicies)).catch((e) => console.error("policies fetch failed", e));
  }, [wallet.id]);

  async function handleFund() {
    setFundLoading(true);
    try {
      const res = await apiPost(`/api/wallets/${wallet.id}/fund`, { to: fundTo, amount: fundAmount, chain: fundChain });
      const data = await res.json();
      setMsg(`Funded! Tx: ${data.tx_hash}`);
      setShowFund(false);
      const p = await apiGet(`/api/wallets/${wallet.id}/portfolio`).then((r) => r.json());
      setPortfolio(p);
      const t = await apiGet(`/api/wallets/${wallet.id}/transactions`).then((r) => r.json());
      setTxs(t);
    } catch (e: unknown) {
      setMsg(`Fund failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setFundLoading(false);
    }
  }

  async function handleCreateKey() {
    setKeyLoading(true);
    try {
      const res = await apiPost(`/api/wallets/${wallet.id}/api-keys`, { name: keyName });
      const data = await res.json();
      setKeyName("");
      setShowKey(false);
      setNewApiKeyToken(data.raw_token || null);
      const k = await apiGet(`/api/wallets/${wallet.id}/api-keys`).then((r) => r.json());
      setKeys(k);
    } catch (e: unknown) {
      setMsg(`API key failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setKeyLoading(false);
    }
  }

  async function handleAnchor() {
    setAnchorLoading(true);
    try {
      const res = await apiPost(`/api/wallets/${wallet.id}/anchor`, {});
      const data = await res.json();
      if (data.tx_hash) {
        setMsg(`Anchored! Tx: ${data.tx_hash}`);
      } else {
        setMsg(data.message || "No unanchored logs");
      }
      const t = await apiGet(`/api/wallets/${wallet.id}/transactions`).then((r) => r.json());
      setTxs(t);
    } catch (e: unknown) {
      setMsg(`Anchor failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setAnchorLoading(false);
    }
  }

  async function handleSwap() {
    setSwapLoading(true);
    try {
      const res = await apiPost(`/api/wallets/${wallet.id}/swap`, {
        chain: swapChain,
        from_token: swapFrom,
        to_token: swapTo,
        amount: swapAmount,
      });
      const data = await res.json();
      if (data.error) {
        setMsg(`Swap denied: ${data.error}`);
      } else {
        setMsg(`Swapped! Tx: ${data.tx_hash}`);
        setShowSwap(false);
        const t = await apiGet(`/api/wallets/${wallet.id}/transactions`).then((r) => r.json());
        setTxs(t);
      }
    } catch (e: unknown) {
      setMsg(`Swap failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setSwapLoading(false);
    }
  }

  const addressMap = new Map<string, string>();
  addresses.forEach((a) => {
    if (!addressMap.has(a.chain_id)) addressMap.set(a.chain_id, a.address);
  });

  function parseNativeBalance(hex: string, chainId: string) {
    const val = BigInt(hex || "0x0");
    if (val === BigInt(0)) {
      if (chainId.startsWith("solana:")) return "0 SOL";
      if (chainId.startsWith("ton:")) return "0 TON";
      if (chainId.startsWith("cfx:")) return "0 CFX";
      return "0 ETH";
    }
    if (chainId.startsWith("solana:")) {
      const sol = Number(val) / 1e9;
      return `${sol.toFixed(6)} SOL`;
    }
    if (chainId.startsWith("ton:")) {
      const ton = Number(val) / 1e9;
      return `${ton.toFixed(6)} TON`;
    }
    if (chainId.startsWith("cfx:")) {
      const cfx = Number(val) / 1e18;
      return `${cfx.toFixed(6)} CFX`;
    }
    const eth = Number(val) / 1e18;
    return `${eth.toFixed(6)} ETH`;
  }

  function copy(text: string) {
    navigator.clipboard.writeText(text).then(() => {
      setMsg("Copied!");
      setTimeout(() => setMsg(""), 1500);
    });
  }

  const btnClass = "text-sm border px-3 py-1 rounded transition-colors";
  const inputClass = "border rounded px-2 py-1 text-sm";

  return (
    <div className="rounded-xl p-5 shadow-sm" style={{ backgroundColor: "var(--card)", border: "1px solid var(--border)" }}>
      <div className="flex justify-between items-start">
        <div>
          <p className="font-semibold text-lg">{wallet.name}</p>
          <p className="text-sm font-mono mt-0.5" style={{ color: "var(--muted-foreground)" }}>{wallet.id}</p>
        </div>
        <div className="flex gap-2 flex-wrap justify-end">
          <button onClick={() => setShowFund(true)} className={btnClass} style={{ borderColor: "var(--border)", backgroundColor: "var(--muted)" }}>Fund</button>
          <button onClick={() => setShowSwap(true)} className={btnClass} style={{ borderColor: "var(--border)", backgroundColor: "var(--muted)" }}>Swap</button>
          <button onClick={() => setShowKey(true)} className={btnClass} style={{ borderColor: "var(--border)", backgroundColor: "var(--muted)" }}>API Key</button>
          <button onClick={handleAnchor} disabled={anchorLoading} className={`${btnClass} disabled:opacity-50`} style={{ borderColor: "var(--border)", backgroundColor: "var(--muted)" }}>
            {anchorLoading ? "Anchoring..." : "Anchor"}
          </button>
        </div>
      </div>

      <div className="mt-4">
        <p className="text-sm font-medium" style={{ color: "var(--foreground)" }}>Addresses</p>
        {addressMap.size === 0 && <p className="text-xs mt-1" style={{ color: "var(--muted-foreground)" }}>No addresses loaded.</p>}
        <div className="mt-2 grid grid-cols-1 gap-2">
          {Array.from(addressMap.entries()).map(([chain, addr]) => (
            <div key={chain} className="rounded-lg px-3 py-2" style={{ backgroundColor: "var(--muted)" }}>
              <div className="flex items-center gap-2">
                <span className="text-xs px-1.5 py-0.5 rounded" style={{ backgroundColor: "var(--card)", color: "var(--primary)", border: "1px solid var(--border)" }}>
                  {formatChainName(chain)}
                </span>
                <span className="text-xs" style={{ color: "var(--muted-foreground)" }}>{chain}</span>
              </div>
              <div className="flex items-center justify-between gap-3 mt-1">
                <p className="text-sm font-mono break-all" style={{ color: "var(--foreground)" }}>{addr}</p>
                <button onClick={() => copy(addr)} className="shrink-0 text-xs px-2 py-1 rounded" style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}>Copy</button>
              </div>
            </div>
          ))}
        </div>
      </div>

      <div className="mt-4">
        <div className="flex items-center justify-between">
          <p className="text-sm font-medium" style={{ color: "var(--foreground)" }}>Policies</p>
          <a
            href="/policies"
            className="text-xs px-2 py-1 rounded"
            style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}
          >
            Manage
          </a>
        </div>
        {policies.length === 0 && <p className="text-xs mt-1" style={{ color: "var(--muted-foreground)" }}>No policies.</p>}
        <div className="mt-2 space-y-2">
          {policies.map((p: Policy) => (
            <div key={p.id} className="rounded-lg px-3 py-2 text-sm" style={{ backgroundColor: "var(--muted)" }}>
              <span className="font-medium">{p.name}</span>
              <span className="text-xs ml-2" style={{ color: "var(--muted-foreground)" }}>
                {formatPolicySummary(p.rules_json)}
              </span>
            </div>
          ))}
        </div>
      </div>

      <div className="mt-4">
        <p className="text-sm font-medium" style={{ color: "var(--foreground)" }}>Balances</p>
        {portfolio.length === 0 && <p className="text-xs mt-1" style={{ color: "var(--muted-foreground)" }}>No balances loaded.</p>}
        <div className="mt-2 grid grid-cols-1 gap-3">
          {portfolio.map((p) => (
            <div key={p.chain_id} className="rounded-lg p-3" style={{ backgroundColor: "var(--muted)" }}>
              <div className="flex items-center gap-2">
                <span className="text-xs px-1.5 py-0.5 rounded" style={{ backgroundColor: "var(--card)", color: "var(--primary)", border: "1px solid var(--border)" }}>
                  {formatChainName(p.chain_id)}
                </span>
                <span className="text-xs font-mono" style={{ color: "var(--muted-foreground)" }}>{parseNativeBalance(p.native_balance, p.chain_id)}</span>
              </div>
              {p.assets.length > 0 && (
                <div className="mt-2 grid grid-cols-2 sm:grid-cols-3 gap-2">
                  {p.assets.map((asset) => (
                    <div key={asset.token_address} className="rounded px-2 py-1.5 text-sm" style={{ backgroundColor: "var(--card)" }}>
                      <div className="font-medium">{asset.symbol}</div>
                      <div className="text-xs font-mono" style={{ color: "var(--muted-foreground)" }}>{asset.balance_formatted}</div>
                    </div>
                  ))}
                </div>
              )}
              {p.assets.length === 0 && <p className="text-xs mt-1" style={{ color: "var(--muted-foreground)" }}>No token assets on this chain.</p>}
            </div>
          ))}
        </div>
      </div>

      <div className="mt-4">
        <p className="text-sm font-medium" style={{ color: "var(--foreground)" }}>Recent Transactions</p>
        {txs.length === 0 && <p className="text-xs mt-1" style={{ color: "var(--muted-foreground)" }}>No transactions.</p>}
        <ul className="text-sm space-y-1 mt-1">
          {txs.map((t) => (
            <li key={t.id} className="flex justify-between items-center rounded-lg px-3 py-1.5" style={{ backgroundColor: "var(--muted)" }}>
              <span>{t.action}</span>
              <span className="text-xs" style={{ color: "var(--muted-foreground)" }}>{t.tx_hash || t.decision}</span>
            </li>
          ))}
        </ul>
      </div>

      <div className="mt-4">
        <p className="text-sm font-medium" style={{ color: "var(--foreground)" }}>API Keys</p>

        {newApiKeyToken && (
          <div className="mt-2 rounded-lg px-3 py-2" style={{ backgroundColor: "#D6EAF8", border: "1px solid #AED6F1" }}>
            <p className="text-xs font-medium" style={{ color: "#1F618D" }}>New API Key created — copy it now, it will not be shown again:</p>
            <div className="flex items-center gap-2 mt-1">
              <code className="text-sm font-mono break-all" style={{ color: "#1F618D" }}>{newApiKeyToken}</code>
              <button onClick={() => copy(newApiKeyToken)} className="text-xs px-2 py-1 rounded" style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}>Copy</button>
              <button onClick={() => setNewApiKeyToken(null)} className="text-xs px-2 py-1 rounded" style={{ backgroundColor: "var(--muted)", color: "var(--muted-foreground)" }}>Hide</button>
            </div>
          </div>
        )}

        {keys.length === 0 && !newApiKeyToken && <p className="text-xs mt-1" style={{ color: "var(--muted-foreground)" }}>No API keys.</p>}
        <div className="flex flex-wrap gap-2 mt-2">
          {keys.map((k) => (
            <span key={k.id} className="text-xs px-2 py-1 rounded" style={{ backgroundColor: "var(--muted)", color: "var(--muted-foreground)" }}>
              {k.name} {k.expired && "(revoked)"}
            </span>
          ))}
        </div>
      </div>

      {showFund && (
        <div className="mt-4 border-t pt-3" style={{ borderColor: "var(--border)" }}>
          <div className="flex gap-2 flex-wrap items-center">
            <select
              className={`${inputClass}`}
              style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
              value={fundChain}
              onChange={(e) => {
                const c = e.target.value;
                setFundChain(c);
                if (c === "solana" || c === "ton" || c === "conflux-core") setFundAmount("0.01");
                else setFundAmount("0.001");
              }}
            >
              <option value="base">Base</option>
              <option value="conflux">Conflux eSpace</option>
              <option value="conflux-core">Conflux Core</option>
              <option value="solana">Solana</option>
              <option value="ton">TON</option>
            </select>
            <input className={`${inputClass} flex-1 min-w-[8rem]`} style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }} placeholder={fundChain === "solana" ? "Solana address" : fundChain === "ton" ? "TON address" : fundChain === "conflux-core" ? "cfxtest:..." : "0x..."} value={fundTo} onChange={(e) => setFundTo(e.target.value)} />
            <input className={`${inputClass} w-24`} style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }} value={fundAmount} onChange={(e) => setFundAmount(e.target.value)} />
            <button onClick={handleFund} disabled={fundLoading} className="px-3 py-1 rounded text-sm disabled:opacity-50" style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}>{fundLoading ? "Sending..." : "Send"}</button>
          </div>
        </div>
      )}

      {showKey && (
        <div className="mt-4 border-t pt-3" style={{ borderColor: "var(--border)" }}>
          <div className="flex gap-2">
            <input className={`${inputClass} flex-1`} style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }} placeholder="Key name" value={keyName} onChange={(e) => setKeyName(e.target.value)} />
            <button onClick={handleCreateKey} disabled={keyLoading} className="px-3 py-1 rounded text-sm disabled:opacity-50" style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}>{keyLoading ? "Creating..." : "Create"}</button>
          </div>
        </div>
      )}

      {showSwap && (
        <div className="mt-4 border-t pt-3" style={{ borderColor: "var(--border)" }}>
          <div className="flex gap-2 flex-wrap items-center">
            <select
              className={`${inputClass}`}
              style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
              value={swapChain}
              onChange={(e) => {
                const c = e.target.value;
                setSwapChain(c);
                if (c === "solana") {
                  setSwapFrom("SOL");
                  setSwapTo("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
                  setSwapAmount("0.1");
                } else if (c === "ton") {
                  setSwapFrom("TON");
                  setSwapTo("Swap DEX TBD");
                  setSwapAmount("0.1");
                } else if (c === "conflux-core") {
                  setSwapFrom("CFX");
                  setSwapTo("Swap DEX TBD");
                  setSwapAmount("0.1");
                } else {
                  setSwapFrom("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913");
                  setSwapTo("0x4200000000000000000000000000000000000006");
                  setSwapAmount("1");
                }
              }}
            >
              <option value="base">Base</option>
              <option value="conflux">Conflux eSpace</option>
              <option value="conflux-core">Conflux Core</option>
              <option value="solana">Solana</option>
              <option value="ton">TON</option>
            </select>
            <input className={`${inputClass} flex-1 min-w-[8rem]`} style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }} placeholder={swapChain === "solana" ? "From token / mint" : swapChain === "ton" ? "From token" : swapChain === "conflux-core" ? "From token" : "From token"} value={swapFrom} onChange={(e) => setSwapFrom(e.target.value)} />
            <input className={`${inputClass} flex-1 min-w-[8rem]`} style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }} placeholder={swapChain === "solana" ? "To token / mint" : swapChain === "ton" ? "To token" : swapChain === "conflux-core" ? "To token" : "To token"} value={swapTo} onChange={(e) => setSwapTo(e.target.value)} />
            <input className={`${inputClass} w-24`} style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }} value={swapAmount} onChange={(e) => setSwapAmount(e.target.value)} />
            <button onClick={handleSwap} disabled={swapLoading} className="px-3 py-1 rounded text-sm disabled:opacity-50" style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}>{swapLoading ? "Swapping..." : "Swap"}</button>
          </div>
        </div>
      )}

      {msg && <p className="text-xs mt-2" style={{ color: "#B45309" }}>{msg}</p>}
    </div>
  );
}

function formatPolicySummary(rulesJson: string): string {
  try {
    const obj = JSON.parse(rulesJson);
    const rules = obj.rules || [];
    const parts: string[] = [];
    for (const r of rules) {
      if (r.type === "chain_whitelist" && r.chain_ids?.length) {
        parts.push(`Chains: ${r.chain_ids.join(", ")}`);
      }
      if (r.type === "spend_limit") {
        const eth = (parseInt(r.max, 10) / 1e18).toFixed(4);
        parts.push(`Limit: ${eth} ${r.token || "ETH"}`);
      }
      if (r.type === "contract_whitelist" && r.contracts?.length) {
        parts.push(`Contracts: ${r.contracts.length}`);
      }
      if (r.type === "operation_type" && r.allowed?.length) {
        parts.push(`Ops: ${r.allowed.join(", ")}`);
      }
      if (r.type === "time_window") {
        parts.push(`Window: ${r.start_hour}-${r.end_hour} ${r.timezone}`);
      }
    }
    return parts.join(" | ") || "No rules";
  } catch {
    return "Invalid policy";
  }
}
