"use client";

import { useEffect, useState } from "react";
import { apiGet, apiPost } from "@/lib/api";
import Link from "next/link";

interface Wallet {
  id: string;
  name: string;
}

interface AgentSession {
  id: string;
  wallet_id: string;
  name: string;
  session_type: string;
  agent_key_hash?: string;
  status: string;
  expires_at: string;
  created_at: string;
  boundaries_json?: string;
}

export default function AgentsPage() {
  const [wallets, setWallets] = useState<Wallet[]>([]);
  const [walletId, setWalletId] = useState("");
  const [sessions, setSessions] = useState<AgentSession[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

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

  useEffect(() => {
    if (!walletId) return;
    setError(null);
    setLoading(true);
    apiGet(`/api/agents/sessions?wallet_id=${encodeURIComponent(walletId)}`)
      .then(async (res) => {
        if (res.ok) {
          const data = (await res.json()) as { sessions?: AgentSession[] };
          setSessions(data.sessions || []);
        } else {
          const text = await res.text();
          setError(text);
        }
      })
      .catch((err) => setError(err instanceof Error ? err.message : String(err)))
      .finally(() => setLoading(false));
  }, [walletId]);

  const handleRevoke = async (id: string) => {
    if (!confirm("Revoke this agent session?")) return;
    try {
      const res = await apiPost(`/api/agents/sessions/${encodeURIComponent(id)}/revoke`, {});
      if (!res.ok) throw new Error(await res.text());
      setSessions((prev) =>
        prev.map((s) => (s.id === id ? { ...s, status: "revoked" } : s))
      );
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const formatExpiry = (iso: string) => {
    const d = new Date(iso);
    return d.toLocaleString();
  };

  return (
    <div
      className="min-h-screen p-6 max-w-4xl mx-auto"
      style={{ backgroundColor: "var(--background)", color: "var(--foreground)" }}
    >
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold">Agent Sessions</h1>
        <Link
          href="/agents/new"
          className="px-4 py-2 rounded text-sm font-medium disabled:opacity-50"
          style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}
        >
          + New Agent Session
        </Link>
      </div>

      <div className="mb-6">
        <label className="block text-sm font-medium mb-1">Wallet</label>
        <select
          className="w-full border rounded px-3 py-2"
          style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
          value={walletId}
          onChange={(e) => setWalletId(e.target.value)}
        >
          {wallets.map((w) => (
            <option key={w.id} value={w.id}>
              {w.name} ({w.id.slice(0, 8)}...)
            </option>
          ))}
        </select>
      </div>

      {error && <div className="text-sm mb-4" style={{ color: "#B45309" }}>{error}</div>}

      {loading ? (
        <p className="text-sm" style={{ color: "var(--muted-foreground)" }}>Loading...</p>
      ) : sessions.length === 0 ? (
        <p className="text-sm" style={{ color: "var(--muted-foreground)" }}>
          No agent sessions yet.{" "}
          <Link href="/agents/new" className="underline">
            Create one
          </Link>
          .
        </p>
      ) : (
        <div className="space-y-3">
          {sessions.map((s) => (
            <div
              key={s.id}
              className="border rounded p-4 flex items-center justify-between"
              style={{ borderColor: "var(--border)", backgroundColor: "var(--card)" }}
            >
              <div>
                <div className="flex items-center gap-2">
                  <p className="font-medium">{s.name}</p>
                  <span
                    className="text-xs px-2 py-0.5 rounded"
                    style={{
                      backgroundColor:
                        s.status === "active"
                          ? "var(--muted)"
                          : s.status === "revoked"
                          ? "#F3F4F6"
                          : "var(--muted)",
                      color: s.status === "revoked" ? "#9CA3AF" : "var(--foreground)",
                    }}
                  >
                    {s.status}
                  </span>
                </div>
                <p className="text-xs mt-1" style={{ color: "var(--muted-foreground)" }}>
                  Expires {formatExpiry(s.expires_at)} · {s.session_type.replace(/_/g, " ")}
                </p>
              </div>
              <div className="flex items-center gap-3">
                {s.status === "active" ? (
                  <button
                    onClick={() => handleRevoke(s.id)}
                    className="text-sm px-3 py-1 rounded border"
                    style={{ borderColor: "var(--border)" }}
                  >
                    Revoke
                  </button>
                ) : (
                  <span className="text-sm" style={{ color: "var(--muted-foreground)" }}>
                    Revoked
                  </span>
                )}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
