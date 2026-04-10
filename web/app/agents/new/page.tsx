"use client";

import { useEffect, useState } from "react";
import { apiGet, apiPost } from "@/lib/api";
import Link from "next/link";

interface Wallet {
  id: string;
  name: string;
}

interface LimitRow {
  limit_type: "per_tx" | "daily" | "total";
  token: string;
  amount_raw: string;
}

const CHAINS = [
  { id: "eip155:8453", label: "Base" },
  { id: "eip155:1", label: "Ethereum" },
  { id: "solana:5eykt4UsFv7PfaMuL6CwuHAJ8hHVdDf1b68zZNpXLJWB", label: "Solana" },
  { id: "ton:-239", label: "TON" },
];

const ACTIONS = ["transfer", "swap", "stake", "pay"];

export default function NewAgentSessionPage() {
  const [wallets, setWallets] = useState<Wallet[]>([]);
  const [walletId, setWalletId] = useState("");
  const [name, setName] = useState("");
  const [selectedChains, setSelectedChains] = useState<string[]>(["eip155:8453"]);
  const [selectedActions, setSelectedActions] = useState<string[]>(["transfer"]);
  const [limits, setLimits] = useState<LimitRow[]>([
    { limit_type: "per_tx", token: "ETH", amount_raw: "1000000000000000000" },
  ]);
  const [expiresHours, setExpiresHours] = useState(24);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [created, setCreated] = useState<{ session_id: string; token: string } | null>(null);

  useEffect(() => {
    apiGet("/api/wallets")
      .then(async (res) => {
        if (res.ok) {
          const data = (await res.json()) as { wallets?: Wallet[] };
          const list = data.wallets || [];
          setWallets(list);
          if (list.length > 0) setWalletId(list[0].id);
        }
      })
      .catch(() => {});
  }, []);

  const toggleChain = (id: string) => {
    setSelectedChains((prev) =>
      prev.includes(id) ? prev.filter((c) => c !== id) : [...prev, id]
    );
  };

  const toggleAction = (a: string) => {
    setSelectedActions((prev) =>
      prev.includes(a) ? prev.filter((x) => x !== a) : [...prev, a]
    );
  };

  const updateLimit = (index: number, field: keyof LimitRow, value: string) => {
    setLimits((prev) =>
      prev.map((row, i) => (i === index ? { ...row, [field]: value } : row))
    );
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!walletId || !name.trim()) return;
    setLoading(true);
    setError(null);
    try {
      const res = await apiPost("/api/agents/sessions", {
        wallet_id: walletId,
        name: name.trim(),
        allowed_chains: selectedChains,
        allowed_actions: selectedActions,
        spend_limits: limits,
        contract_whitelist: null,
        expires_hours: expiresHours,
      });
      if (!res.ok) throw new Error(await res.text());
      const data = (await res.json()) as { session_id: string; token: string };
      setCreated(data);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  if (created) {
    return (
      <div
        className="min-h-screen p-6 max-w-xl mx-auto flex flex-col justify-center"
        style={{ backgroundColor: "var(--background)", color: "var(--foreground)" }}
      >
        <div
          className="border rounded p-6 space-y-4"
          style={{ borderColor: "var(--border)", backgroundColor: "var(--card)" }}
        >
          <h1 className="text-xl font-bold">Agent Session Created</h1>
          <p className="text-sm" style={{ color: "var(--muted-foreground)" }}>
            Copy the token below and paste it into your agent configuration. We will not show it again.
          </p>
          <div className="p-3 rounded text-sm break-all font-mono"
            style={{ backgroundColor: "var(--muted)", color: "var(--foreground)" }}
          >
            {created.token}
          </div>
          <div className="flex gap-3">
            <button
              onClick={() => navigator.clipboard.writeText(created.token)}
              className="px-4 py-2 rounded text-sm font-medium"
              style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}
            >
              Copy Token
            </button>
            <Link
              href="/agents"
              className="px-4 py-2 rounded text-sm border"
              style={{ borderColor: "var(--border)" }}
            >
              Done
            </Link>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div
      className="min-h-screen p-6 max-w-xl mx-auto"
      style={{ backgroundColor: "var(--background)", color: "var(--foreground)" }}
    >
      <h1 className="text-2xl font-bold mb-6">Create Agent Session</h1>

      <form onSubmit={handleSubmit} className="space-y-5">
        <div>
          <label className="block text-sm font-medium mb-1">Name</label>
          <input
            className="w-full border rounded px-3 py-2"
            style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
            placeholder="e.g. Hardness DeFi Bot"
            value={name}
            onChange={(e) => setName(e.target.value)}
            required
          />
        </div>

        <div>
          <label className="block text-sm font-medium mb-1">Wallet</label>
          <select
            className="w-full border rounded px-3 py-2"
            style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
            value={walletId}
            onChange={(e) => setWalletId(e.target.value)}
            required
          >
            {wallets.map((w) => (
              <option key={w.id} value={w.id}>
                {w.name} ({w.id.slice(0, 8)}...)
              </option>
            ))}
          </select>
        </div>

        <div>
          <label className="block text-sm font-medium mb-2">Allowed Chains</label>
          <div className="flex flex-wrap gap-3">
            {CHAINS.map((c) => (
              <label key={c.id} className="flex items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  checked={selectedChains.includes(c.id)}
                  onChange={() => toggleChain(c.id)}
                />
                {c.label}
              </label>
            ))}
          </div>
        </div>

        <div>
          <label className="block text-sm font-medium mb-2">Allowed Actions</label>
          <div className="flex flex-wrap gap-3">
            {ACTIONS.map((a) => (
              <label key={a} className="flex items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  checked={selectedActions.includes(a)}
                  onChange={() => toggleAction(a)}
                />
                {a}
              </label>
            ))}
          </div>
        </div>

        <div>
          <label className="block text-sm font-medium mb-2">Spend Limits</label>
          <div className="space-y-2">
            {limits.map((row, idx) => (
              <div key={idx} className="flex gap-2">
                <select
                  className="border rounded px-2 py-1 text-sm"
                  style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
                  value={row.limit_type}
                  onChange={(e) => updateLimit(idx, "limit_type", e.target.value)}
                >
                  <option value="per_tx">per tx</option>
                  <option value="daily">daily</option>
                  <option value="total">total</option>
                </select>
                <input
                  className="border rounded px-2 py-1 text-sm flex-1"
                  style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
                  placeholder="Token (e.g. ETH)"
                  value={row.token}
                  onChange={(e) => updateLimit(idx, "token", e.target.value)}
                />
                <input
                  className="border rounded px-2 py-1 text-sm flex-1"
                  style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
                  placeholder="Amount raw"
                  value={row.amount_raw}
                  onChange={(e) => updateLimit(idx, "amount_raw", e.target.value)}
                />
                <button
                  type="button"
                  onClick={() => setLimits((prev) => prev.filter((_, i) => i !== idx))}
                  className="text-sm px-2 rounded border"
                  style={{ borderColor: "var(--border)" }}
                >
                  −
                </button>
              </div>
            ))}
            <button
              type="button"
              onClick={() =>
                setLimits((prev) => [...prev, { limit_type: "per_tx", token: "ETH", amount_raw: "" }])
              }
              className="text-sm px-3 py-1 rounded border"
              style={{ borderColor: "var(--border)" }}
            >
              + Add limit
            </button>
          </div>
        </div>

        <div>
          <label className="block text-sm font-medium mb-1">Expires in (hours)</label>
          <input
            type="number"
            min={1}
            className="w-full border rounded px-3 py-2"
            style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
            value={expiresHours}
            onChange={(e) => setExpiresHours(parseInt(e.target.value) || 1)}
            required
          />
        </div>

        {error && <div className="text-sm" style={{ color: "#B45309" }}>{error}</div>}

        <div className="flex gap-3 pt-2">
          <button
            type="submit"
            disabled={loading || !walletId || !name.trim()}
            className="px-4 py-2 rounded font-medium disabled:opacity-50"
            style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}
          >
            {loading ? "Creating..." : "Create Session"}
          </button>
          <Link
            href="/agents"
            className="px-4 py-2 rounded border"
            style={{ borderColor: "var(--border)" }}
          >
            Cancel
          </Link>
        </div>
      </form>
    </div>
  );
}
