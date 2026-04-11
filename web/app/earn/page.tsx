"use client";

import { useEffect, useState } from "react";
import { apiGet, apiPost } from "@/lib/api";

interface UnderlyingToken {
  symbol: string;
  address: string;
  decimals: number;
}

interface VaultAnalytics {
  apy?: {
    base?: number;
    total?: number;
    reward?: number | null;
  };
  tvl?: {
    usd?: string;
  };
}

interface Vault {
  name: string;
  address: string;
  chainId: number;
  protocol?: {
    name: string;
    url?: string;
  };
  analytics?: VaultAnalytics;
  underlyingTokens?: UnderlyingToken[];
  description?: string;
}

interface Wallet {
  id: string;
  name: string;
}

export default function EarnPage() {
  const [vaults, setVaults] = useState<Vault[]>([]);
  const [wallets, setWallets] = useState<Wallet[]>([]);
  const [selectedWalletId, setSelectedWalletId] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [msg, setMsg] = useState("");
  const [depositing, setDepositing] = useState<Record<string, boolean>>({});
  const [amounts, setAmounts] = useState<Record<string, string>>({});

  useEffect(() => {
    apiGet("/api/wallets")
      .then(async (res) => {
        const data = await res.json();
        const list = data as Wallet[];
        setWallets(list);
        if (list.length > 0) {
          setSelectedWalletId(list[0].id);
        }
      })
      .catch(() => setMsg(""));
  }, []);

  async function discover() {
    setLoading(true);
    setMsg("");
    try {
      const res = await apiGet("/api/earn/discover?chain_id=8453");
      if (!res.ok) {
        throw new Error(`HTTP ${res.status}`);
      }
      const data = await res.json();
      setVaults(data.vaults || []);
    } catch (e: unknown) {
      setMsg(`Failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setLoading(false);
    }
  }

  async function deposit(vault: Vault) {
    if (!selectedWalletId) {
      setMsg("Please select a wallet first.");
      return;
    }
    const token = vault.underlyingTokens?.[0];
    if (!token) {
      setMsg("No underlying token info for this vault.");
      return;
    }
    const raw = amounts[vault.address] || "";
    if (!raw || Number(raw) <= 0) {
      setMsg("Enter a valid amount.");
      return;
    }
    const scale = BigInt("1" + "0".repeat(token.decimals));
    const amount = (BigInt(Math.floor(Number(raw) * 1e6)) * scale / BigInt(1e6)).toString();

    setDepositing((d) => ({ ...d, [vault.address]: true }));
    setMsg("");
    try {
      const res = await apiPost(`/api/wallets/${selectedWalletId}/earn-deposit`, {
        vault_address: vault.address,
        from_token: token.address,
        amount,
      });
      const data = await res.json();
      setMsg(`Deposit submitted! tx_hash: ${data.tx_hash}`);
    } catch (e: unknown) {
      const text = e instanceof Error ? e.message : String(e);
      setMsg(`Deposit failed: ${text}`);
    } finally {
      setDepositing((d) => ({ ...d, [vault.address]: false }));
    }
  }

  return (
    <div className="min-h-screen p-6" style={{ backgroundColor: "var(--background)", color: "var(--foreground)" }}>
      <div className="max-w-3xl mx-auto">
        <h1 className="text-3xl font-bold mb-2">Earn</h1>
        <p className="mb-4" style={{ color: "var(--muted-foreground)" }}>
          Discover yield vaults and deposit directly from your wallet.
        </p>

        <div className="mb-4 flex items-center gap-3">
          <label className="text-sm" style={{ color: "var(--muted-foreground)" }}>Wallet:</label>
          <select
            className="rounded-md border px-3 py-2 text-sm"
            style={{ borderColor: "var(--border)", backgroundColor: "var(--card)" }}
            value={selectedWalletId}
            onChange={(e) => setSelectedWalletId(e.target.value)}
          >
            {wallets.map((w) => (
              <option key={w.id} value={w.id}>{w.name}</option>
            ))}
          </select>
        </div>

        <button
          onClick={discover}
          disabled={loading}
          className="px-5 py-2.5 rounded-lg font-semibold text-white transition disabled:opacity-60"
          style={{ backgroundColor: "var(--primary)" }}
        >
          {loading ? "Discovering..." : "Discover Base Vaults"}
        </button>

        {msg && <p className="mt-4 text-sm text-red-500">{msg}</p>}

        <div className="mt-6 space-y-4">
          {vaults.map((v) => {
            const apy = v.analytics?.apy?.total ?? v.analytics?.apy?.base ?? null;
            const tvl = v.analytics?.tvl?.usd;
            const token = v.underlyingTokens?.[0];
            return (
              <div
                key={v.address}
                className="rounded-xl border p-4 transition"
                style={{ borderColor: "var(--border)", backgroundColor: "var(--card)" }}
              >
                <div className="flex items-center justify-between">
                  <div className="font-semibold text-lg">{v.name}</div>
                  {apy !== null && (
                    <div className="text-sm font-medium px-2 py-1 rounded" style={{ backgroundColor: "var(--muted)", color: "var(--primary)" }}>
                      APY: {apy}%
                    </div>
                  )}
                </div>
                <div className="mt-1 text-sm" style={{ color: "var(--muted-foreground)" }}>
                  Protocol: {v.protocol?.name || "Unknown"} · Chain: Base
                </div>
                {tvl && (
                  <div className="text-sm" style={{ color: "var(--muted-foreground)" }}>
                    TVL: ${Number(tvl).toLocaleString()}
                  </div>
                )}
                {token && (
                  <div className="text-sm" style={{ color: "var(--muted-foreground)" }}>
                    Asset: {token.symbol} ({token.address.slice(0, 6)}...{token.address.slice(-4)})
                  </div>
                )}
                {v.description && (
                  <div className="mt-2 text-sm" style={{ color: "var(--muted-foreground)" }}>
                    {v.description}
                  </div>
                )}
                <div className="mt-3 text-xs font-mono break-all" style={{ color: "var(--muted-foreground)" }}>
                  Vault: {v.address}
                </div>

                {token && (
                  <div className="mt-4 flex items-center gap-2">
                    <input
                      type="number"
                      placeholder={`Amount in ${token.symbol}`}
                      className="flex-1 rounded-md border px-3 py-2 text-sm"
                      style={{ borderColor: "var(--border)", backgroundColor: "var(--card)" }}
                      value={amounts[v.address] || ""}
                      onChange={(e) => setAmounts((a) => ({ ...a, [v.address]: e.target.value }))}
                    />
                    <button
                      onClick={() => deposit(v)}
                      disabled={depositing[v.address]}
                      className="px-4 py-2 rounded-md font-semibold text-white transition disabled:opacity-60"
                      style={{ backgroundColor: "var(--primary)" }}
                    >
                      {depositing[v.address] ? "Deposit..." : "Deposit"}
                    </button>
                  </div>
                )}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
