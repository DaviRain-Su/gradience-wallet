"use client";

import { useState } from "react";
import { apiGet } from "@/lib/api";

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

export default function EarnPage() {
  const [vaults, setVaults] = useState<Vault[]>([]);
  const [loading, setLoading] = useState(false);
  const [msg, setMsg] = useState("");

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

  return (
    <div className="min-h-screen p-6" style={{ backgroundColor: "var(--background)", color: "var(--foreground)" }}>
      <div className="max-w-3xl mx-auto">
        <h1 className="text-3xl font-bold mb-2">Earn</h1>
        <p className="mb-6" style={{ color: "var(--muted-foreground)" }}>
          Discover yield vaults and deposit directly from your wallet.
        </p>

        <button
          onClick={discover}
          disabled={loading}
          className="px-5 py-2.5 rounded-lg font-semibold text-white transition disabled:opacity-60"
          style={{ backgroundColor: "var(--primary)" }}
        >
          {loading ? "Discovering..." : "Discover Base Vaults"}
        </button>

        {msg && <p className="mt-4 text-red-500">{msg}</p>}

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
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
