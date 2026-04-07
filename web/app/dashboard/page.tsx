"use client";

import { useEffect, useState } from "react";
import { apiGet, apiPost } from "@/lib/api";

interface Wallet {
  id: string;
  name: string;
  owner_id: string;
  workspace_id: string | null;
  status: string;
  created_at: string;
  updated_at: string;
}

interface Balance {
  chain_id: string;
  address: string;
  balance: string;
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

export default function Dashboard() {
  const [wallets, setWallets] = useState<Wallet[]>([]);
  const [name, setName] = useState("");
  const [loading, setLoading] = useState(false);
  const [msg, setMsg] = useState("");

  async function fetchWallets() {
    try {
      const res = await apiGet("/api/wallets");
      const data = await res.json();
      setWallets(data);
    } catch (e: unknown) {
      setMsg(`Failed to load wallets: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  useEffect(() => {
    fetchWallets();
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
    <div className="min-h-screen p-8 max-w-4xl mx-auto">
      <h1 className="text-2xl font-bold mb-6">Wallet Dashboard</h1>

      <div className="flex gap-2 mb-6">
        <input
          className="border rounded px-3 py-2 flex-1"
          placeholder="Wallet name"
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
        <button
          onClick={handleCreate}
          disabled={loading}
          className="bg-black text-white px-4 py-2 rounded disabled:opacity-50"
        >
          {loading ? "Creating..." : "Create Wallet"}
        </button>
      </div>

      {msg && <p className="text-red-500 text-sm mb-4">{msg}</p>}

      <div className="space-y-6">
        {wallets.length === 0 && <p className="text-gray-500">No wallets yet.</p>}
        {wallets.map((w) => (
          <WalletCard key={w.id} wallet={w} />
        ))}
      </div>
    </div>
  );
}

function WalletCard({ wallet }: { wallet: Wallet }) {
  const [balances, setBalances] = useState<Balance[]>([]);
  const [txs, setTxs] = useState<Tx[]>([]);
  const [keys, setKeys] = useState<ApiKey[]>([]);
  const [showFund, setShowFund] = useState(false);
  const [showKey, setShowKey] = useState(false);
  const [fundTo, setFundTo] = useState("");
  const [fundAmount, setFundAmount] = useState("0.001");
  const [keyName, setKeyName] = useState("");
  const [msg, setMsg] = useState("");

  useEffect(() => {
    apiGet(`/api/wallets/${wallet.id}/balance`).then((r) => r.json().then(setBalances)).catch(() => {});
    apiGet(`/api/wallets/${wallet.id}/transactions`).then((r) => r.json().then(setTxs)).catch(() => {});
    apiGet(`/api/wallets/${wallet.id}/api-keys`).then((r) => r.json().then(setKeys)).catch(() => {});
  }, [wallet.id]);

  async function handleFund() {
    try {
      const res = await apiPost(`/api/wallets/${wallet.id}/fund`, { to: fundTo, amount: fundAmount, chain: "base" });
      const data = await res.json();
      setMsg(`Funded! Tx: ${data.tx_hash}`);
      setShowFund(false);
      const b = await apiGet(`/api/wallets/${wallet.id}/balance`).then((r) => r.json());
      setBalances(b);
      const t = await apiGet(`/api/wallets/${wallet.id}/transactions`).then((r) => r.json());
      setTxs(t);
    } catch (e: unknown) {
      setMsg(`Fund failed: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  async function handleCreateKey() {
    try {
      await apiPost(`/api/wallets/${wallet.id}/api-keys`, { name: keyName });
      setKeyName("");
      setShowKey(false);
      const k = await apiGet(`/api/wallets/${wallet.id}/api-keys`).then((r) => r.json());
      setKeys(k);
    } catch (e: unknown) {
      setMsg(`API key failed: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  async function handleAnchor() {
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
    }
  }

  return (
    <div className="border rounded p-4">
      <div className="flex justify-between items-start">
        <div>
          <p className="font-semibold text-lg">{wallet.name}</p>
          <p className="text-sm text-gray-500 font-mono">{wallet.id}</p>
        </div>
        <div className="flex gap-2">
          <button onClick={() => setShowFund(true)} className="text-sm border px-3 py-1 rounded hover:bg-gray-100">Fund</button>
          <button onClick={() => setShowKey(true)} className="text-sm border px-3 py-1 rounded hover:bg-gray-100">API Key</button>
          <button onClick={handleAnchor} className="text-sm border px-3 py-1 rounded hover:bg-gray-100">Anchor</button>
        </div>
      </div>

      <div className="mt-3">
        <p className="text-sm font-medium">Balances</p>
        {balances.length === 0 && <p className="text-xs text-gray-400">No balances loaded.</p>}
        {balances.map((b) => (
          <p key={b.chain_id} className="text-sm">
            {b.chain_id}: <span className="font-mono">{b.balance}</span> ({b.address})
          </p>
        ))}
      </div>

      <div className="mt-3">
        <p className="text-sm font-medium">Recent Transactions</p>
        {txs.length === 0 && <p className="text-xs text-gray-400">No transactions.</p>}
        <ul className="text-sm space-y-1">
          {txs.map((t) => (
            <li key={t.id} className="flex justify-between">
              <span>{t.action}</span>
              <span className="text-gray-500 text-xs">{t.tx_hash || t.decision}</span>
            </li>
          ))}
        </ul>
      </div>

      <div className="mt-3">
        <p className="text-sm font-medium">API Keys</p>
        {keys.length === 0 && <p className="text-xs text-gray-400">No API keys.</p>}
        <div className="flex flex-wrap gap-2">
          {keys.map((k) => (
            <span key={k.id} className="text-xs bg-gray-100 px-2 py-1 rounded">
              {k.name} {k.expired && "(revoked)"}
            </span>
          ))}
        </div>
      </div>

      {showFund && (
        <div className="mt-4 border-t pt-3">
          <div className="flex gap-2">
            <input className="border rounded px-2 py-1 flex-1 text-sm" placeholder="To address" value={fundTo} onChange={(e) => setFundTo(e.target.value)} />
            <input className="border rounded px-2 py-1 w-24 text-sm" value={fundAmount} onChange={(e) => setFundAmount(e.target.value)} />
            <button onClick={handleFund} className="bg-black text-white px-3 py-1 rounded text-sm">Send</button>
          </div>
        </div>
      )}

      {showKey && (
        <div className="mt-4 border-t pt-3">
          <div className="flex gap-2">
            <input className="border rounded px-2 py-1 flex-1 text-sm" placeholder="Key name" value={keyName} onChange={(e) => setKeyName(e.target.value)} />
            <button onClick={handleCreateKey} className="bg-black text-white px-3 py-1 rounded text-sm">Create</button>
          </div>
        </div>
      )}

      {msg && <p className="text-xs text-red-500 mt-2">{msg}</p>}
    </div>
  );
}
