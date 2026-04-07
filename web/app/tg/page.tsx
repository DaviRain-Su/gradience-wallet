"use client";

import { useEffect, useState } from "react";
import { apiGet, apiPost } from "@/lib/api";

interface Wallet {
  id: string;
  name: string;
}

interface Balance {
  chain_id: string;
  address: string;
  balance: string;
}

export default function TgDashboard() {
  const [wallets, setWallets] = useState<Wallet[]>([]);
  const [loading, setLoading] = useState(false);
  const [msg, setMsg] = useState("");
  const [tgUser, setTgUser] = useState<string>("");

  useEffect(() => {
    const user = window.Telegram?.WebApp?.initDataUnsafe?.user;
    if (user?.username) {
      setTgUser(user.username);
    } else if (user?.first_name) {
      setTgUser(user.first_name);
    }
  }, []);

  async function fetchWallets() {
    try {
      const res = await apiGet("/api/wallets");
      const data = await res.json();
      setWallets(data);
    } catch (e: unknown) {
      setMsg(`Failed: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  useEffect(() => {
    fetchWallets();
  }, []);

  async function handleCreate() {
    const name = tgUser ? `${tgUser}-wallet` : "tg-wallet";
    setLoading(true);
    try {
      await apiPost("/api/wallets", { name });
      await fetchWallets();
      window.Telegram?.WebApp?.HapticFeedback?.notificationOccurred("success");
    } catch (e: unknown) {
      setMsg(`Create failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="p-4 max-w-md mx-auto">
      <h1 className="text-xl font-bold mb-4">Gradience Wallet</h1>
      {tgUser && <p className="text-sm text-gray-500 mb-4">Hello, {tgUser}</p>}

      <button
        onClick={handleCreate}
        disabled={loading}
        className="w-full bg-[var(--tg-button-color,#3390ec)] text-[var(--tg-button-text-color,#fff)] py-3 rounded-xl font-medium disabled:opacity-60 mb-4"
      >
        {loading ? "Creating..." : "Create Wallet"}
      </button>

      {msg && <p className="text-red-500 text-sm mb-4">{msg}</p>}

      <div className="space-y-3">
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
  const [fundTo, setFundTo] = useState("");
  const [fundAmount, setFundAmount] = useState("0.001");
  const [showFund, setShowFund] = useState(false);
  const [fundLoading, setFundLoading] = useState(false);
  const [msg, setMsg] = useState("");

  useEffect(() => {
    apiGet(`/api/wallets/${wallet.id}/balance`)
      .then((r) => r.json().then(setBalances))
      .catch(() => {});
  }, [wallet.id]);

  async function handleFund() {
    setFundLoading(true);
    try {
      const res = await apiPost(`/api/wallets/${wallet.id}/fund`, {
        to: fundTo,
        amount: fundAmount,
        chain: "base",
      });
      const data = await res.json();
      setMsg(`Sent! Tx: ${data.tx_hash?.slice(0, 12)}...`);
      setShowFund(false);
      window.Telegram?.WebApp?.HapticFeedback?.impactOccurred("light");
      const b = await apiGet(`/api/wallets/${wallet.id}/balance`).then((r) => r.json());
      setBalances(b);
    } catch (e: unknown) {
      setMsg(`Fund failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setFundLoading(false);
    }
  }

  return (
    <div className="border rounded-xl p-3 bg-white shadow-sm">
      <div className="flex justify-between items-center">
        <p className="font-semibold">{wallet.name}</p>
        <button
          onClick={() => setShowFund((s) => !s)}
          className="text-sm px-3 py-1 rounded-lg bg-gray-100"
        >
          {showFund ? "Close" : "Fund"}
        </button>
      </div>

      <div className="mt-2">
        {balances.length === 0 && <p className="text-xs text-gray-400">No balances.</p>}
        {balances.map((b) => (
          <p key={b.chain_id} className="text-sm">
            {b.chain_id}: <span className="font-mono">{b.balance}</span>
          </p>
        ))}
      </div>

      {showFund && (
        <div className="mt-3 flex gap-2">
          <input
            className="border rounded px-2 py-1 flex-1 text-sm"
            placeholder="To address"
            value={fundTo}
            onChange={(e) => setFundTo(e.target.value)}
          />
          <input
            className="border rounded px-2 py-1 w-20 text-sm"
            value={fundAmount}
            onChange={(e) => setFundAmount(e.target.value)}
          />
          <button
            onClick={handleFund}
            disabled={fundLoading}
            className="bg-[var(--tg-button-color,#3390ec)] text-[var(--tg-button-text-color,#fff)] px-3 py-1 rounded text-sm disabled:opacity-60"
          >
            {fundLoading ? "..." : "Send"}
          </button>
        </div>
      )}

      {msg && <p className="text-xs text-red-500 mt-2">{msg}</p>}
    </div>
  );
}
