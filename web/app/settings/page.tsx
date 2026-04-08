"use client";

import { useEffect, useState } from "react";
import { apiGet, apiDelete } from "@/lib/api";

interface Session {
  token: string;
  username: string;
  created_at: string;
  expires_at: string;
  current: boolean;
}

interface ApiKey {
  id: string;
  name: string;
  permissions: string;
  expired: boolean;
}

interface Wallet {
  id: string;
  name: string;
}

export default function SettingsPage() {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [wallets, setWallets] = useState<Wallet[]>([]);
  const [keysMap, setKeysMap] = useState<Record<string, ApiKey[]>>({});
  const [msg, setMsg] = useState("");
  const [loading, setLoading] = useState(false);

  async function fetchSessions() {
    try {
      const res = await apiGet("/api/auth/me/sessions");
      const data = await res.json();
      setSessions(data);
    } catch (e: unknown) {
      setMsg(`Failed to load sessions: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  async function fetchWalletsAndKeys() {
    try {
      const res = await apiGet("/api/wallets");
      const w: Wallet[] = await res.json();
      setWallets(w);
      const map: Record<string, ApiKey[]> = {};
      await Promise.all(
        w.map(async (wallet) => {
          const kres = await apiGet(`/api/wallets/${wallet.id}/api-keys`);
          const k: ApiKey[] = await kres.json();
          map[wallet.id] = k;
        })
      );
      setKeysMap(map);
    } catch (e: unknown) {
      setMsg(`Failed to load keys: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  useEffect(() => {
    setLoading(true);
    Promise.all([fetchSessions(), fetchWalletsAndKeys()]).finally(() => setLoading(false));
  }, []);

  async function revokeSession(token: string) {
    if (!confirm("Revoke this session?")) return;
    try {
      await apiDelete("/api/auth/sessions", { token });
      setMsg("Session revoked");
      await fetchSessions();
    } catch (e: unknown) {
      setMsg(`Revoke failed: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  async function revokeKey(walletId: string, keyId: string) {
    if (!confirm("Revoke this API key?")) return;
    try {
      await apiDelete(`/api/wallets/${walletId}/api-keys/${keyId}`);
      setMsg("API key revoked");
      await fetchWalletsAndKeys();
    } catch (e: unknown) {
      setMsg(`Revoke failed: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  return (
    <div className="min-h-screen p-8 max-w-3xl mx-auto" style={{ backgroundColor: "var(--background)", color: "var(--foreground)" }}>
      <h1 className="text-2xl font-bold mb-6">Settings</h1>

      {loading && <p style={{ color: "var(--muted-foreground)" }}>Loading...</p>}
      {msg && <p className="text-sm mb-4" style={{ color: "#B45309" }}>{msg}</p>}

      <div className="border rounded-lg p-5 mb-6" style={{ borderColor: "var(--border)", backgroundColor: "var(--card)" }}>
        <h2 className="text-lg font-semibold mb-3">Active Sessions</h2>
        {sessions.length === 0 && <p className="text-sm" style={{ color: "var(--muted-foreground)" }}>No active sessions.</p>}
        <ul className="space-y-3">
          {sessions.map((s) => (
            <li key={s.token} className="flex justify-between items-center border-b pb-2 last:border-0" style={{ borderColor: "var(--border)" }}>
              <div>
                <p className="text-sm font-medium">{s.username}</p>
                <p className="text-xs" style={{ color: "var(--muted-foreground)" }}>
                  {s.current ? "Current session" : `Expires ${new Date(s.expires_at).toLocaleString()}`}
                </p>
              </div>
              {!s.current && (
                <button
                  onClick={() => revokeSession(s.token)}
                  className="text-xs px-2 py-1 rounded border"
                  style={{ borderColor: "var(--border)", backgroundColor: "var(--muted)", color: "var(--foreground)" }}
                >
                  Revoke
                </button>
              )}
            </li>
          ))}
        </ul>
      </div>

      <div className="border rounded-lg p-5" style={{ borderColor: "var(--border)", backgroundColor: "var(--card)" }}>
        <h2 className="text-lg font-semibold mb-3">API Keys</h2>
        {wallets.length === 0 && <p className="text-sm" style={{ color: "var(--muted-foreground)" }}>No wallets.</p>}
        {wallets.map((w) => (
          <div key={w.id} className="mb-4">
            <p className="text-sm font-medium mb-1">{w.name}</p>
            {(keysMap[w.id] || []).length === 0 && (
              <p className="text-xs" style={{ color: "var(--muted-foreground)" }}>No keys.</p>
            )}
            <ul className="space-y-2">
              {(keysMap[w.id] || []).map((k) => (
                <li key={k.id} className="flex justify-between items-center">
                  <div>
                    <p className="text-sm">{k.name}</p>
                    <p className="text-xs" style={{ color: "var(--muted-foreground)" }}>
                      {k.permissions} {k.expired ? "• Expired" : ""}
                    </p>
                  </div>
                  {!k.expired && (
                    <button
                      onClick={() => revokeKey(w.id, k.id)}
                      className="text-xs px-2 py-1 rounded border"
                      style={{ borderColor: "var(--border)", backgroundColor: "var(--muted)", color: "var(--foreground)" }}
                    >
                      Revoke
                    </button>
                  )}
                </li>
              ))}
            </ul>
          </div>
        ))}
      </div>
    </div>
  );
}
