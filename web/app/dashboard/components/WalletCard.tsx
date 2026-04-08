"use client";

import { useEffect, useState } from "react";
import { apiGet, apiPost } from "@/lib/api";
import { formatChainName } from "@/lib/chains";
import type { Wallet, Address, Portfolio, Tx, ApiKey, Policy } from "../types";

export default function WalletCard({ wallet }: { wallet: Wallet }) {
  const [portfolio, setPortfolio] = useState<Portfolio[]>([]);
  const [addresses, setAddresses] = useState<Address[]>([]);
  const [txs, setTxs] = useState<Tx[]>([]);
  const [keys, setKeys] = useState<ApiKey[]>([]);
  const [policies, setPolicies] = useState<Policy[]>([]);
  const [newApiKeyToken, setNewApiKeyToken] = useState<string | null>(null);
  const [showFund, setShowFund] = useState(false);
  const [showKey, setShowKey] = useState(false);
  const [showAllAddresses, setShowAllAddresses] = useState(false);
  const [fundTo, setFundTo] = useState("");
  const [fundAmount, setFundAmount] = useState("0.001");
  const [keyName, setKeyName] = useState("");
  const [msg, setMsg] = useState("");
  const [fundLoading, setFundLoading] = useState(false);
  const [keyLoading, setKeyLoading] = useState(false);
  const [anchorLoading, setAnchorLoading] = useState(false);

  const [fundChain, setFundChain] = useState("base");

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


  const addressMap = new Map<string, string>();
  addresses.forEach((a) => {
    if (!addressMap.has(a.chain_id)) addressMap.set(a.chain_id, a.address);
  });

  function getNativeSymbol(chainId: string) {
    if (chainId.startsWith("solana:")) return "SOL";
    if (chainId.startsWith("ton:")) return "TON";
    if (chainId.startsWith("cfx:")) return "CFX";
    if (chainId.startsWith("cosmos:")) return "ATOM";
    if (chainId.startsWith("filecoin:")) return "FIL";
    if (chainId.startsWith("xrpl:") || chainId.startsWith("xrp:")) return "XRP";
    return "ETH";
  }

  function getNativeDecimals(chainId: string) {
    if (chainId.startsWith("solana:")) return 1e9;
    if (chainId.startsWith("ton:")) return 1e9;
    if (chainId.startsWith("cfx:")) return 1e18;
    if (chainId.startsWith("cosmos:")) return 1e6;
    if (chainId.startsWith("filecoin:")) return 1e18;
    if (chainId.startsWith("xrpl:") || chainId.startsWith("xrp:")) return 1e6;
    return 1e18;
  }

  function parseNativeBalance(hex: string, chainId: string) {
    try {
      const val = BigInt(hex || "0x0");
      const symbol = getNativeSymbol(chainId);
      const dec = getNativeDecimals(chainId);
      if (val === BigInt(0)) return `0 ${symbol}`;
      const amt = Number(val) / dec;
      return `${amt.toFixed(6)} ${symbol}`;
    } catch {
      return `0 ${getNativeSymbol(chainId)}`;
    }
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
          {(showAllAddresses ? Array.from(addressMap.entries()) : Array.from(addressMap.entries()).slice(0, 2)).map(([chain, addr]) => (
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
        {addressMap.size > 2 && (
          <button
            onClick={() => setShowAllAddresses((s) => !s)}
            className="text-xs mt-2 underline"
            style={{ color: "var(--primary)" }}
          >
            {showAllAddresses ? "Collapse addresses" : `Show all addresses (${addressMap.size})`}
          </button>
        )}
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
          {[...portfolio].sort((a, b) => {
            const aVal = BigInt(a.native_balance || "0x0");
            const bVal = BigInt(b.native_balance || "0x0");
            const aScore = (aVal > BigInt(0) ? 2 : 0) + (a.assets.length > 0 ? 1 : 0);
            const bScore = (bVal > BigInt(0) ? 2 : 0) + (b.assets.length > 0 ? 1 : 0);
            return bScore - aScore;
          }).map((p) => (
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
