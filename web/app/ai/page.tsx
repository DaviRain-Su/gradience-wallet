"use client";

import { useEffect, useState } from "react";
import {
  AiProxyKey,
  createAiProxyKey,
  deleteAiProxyKey,
  listAiProxyKeys,
} from "@/lib/mpp";
import { apiGet } from "@/lib/api";

interface Wallet {
  id: string;
  name: string;
}

export default function AiGateway() {
  const [wallets, setWallets] = useState<Wallet[]>([]);
  const [walletId, setWalletId] = useState("");
  const [keys, setKeys] = useState<AiProxyKey[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [newKeyName, setNewKeyName] = useState("");
  const [createdToken, setCreatedToken] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

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
    listAiProxyKeys(walletId)
      .then(setKeys)
      .catch((err) => setError(err instanceof Error ? err.message : String(err)));
  }, [walletId]);

  const handleCreate = async () => {
    if (!walletId || !newKeyName.trim()) return;
    setLoading(true);
    setError(null);
    setCreatedToken(null);
    try {
      const resp = await createAiProxyKey({
        wallet_id: walletId,
        name: newKeyName.trim(),
      });
      setCreatedToken(resp.raw_token);
      setNewKeyName("");
      const updated = await listAiProxyKeys(walletId);
      setKeys(updated);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleDelete = async (keyId: string) => {
    setLoading(true);
    try {
      await deleteAiProxyKey(keyId);
      const updated = await listAiProxyKeys(walletId);
      setKeys(updated);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  const copy = (text: string) => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  const baseUrl = typeof window !== "undefined"
    ? `${window.location.origin.replace(/\/+$/, "")}/v1/proxy/openai`
    : "https://api.gradiences.xyz/v1/proxy/openai";

  return (
    <div
      className="min-h-screen p-6 max-w-3xl mx-auto"
      style={{ backgroundColor: "var(--background)", color: "var(--foreground)" }}
    >
      <h1 className="text-2xl font-bold mb-2">AI Gateway</h1>
      <p className="text-sm mb-6" style={{ color: "var(--muted-foreground)" }}>
        Connect any AI agent to Gradience via a temporary API key. Pay per request on-chain through MPP.
      </p>

      <div className="space-y-4">
        <div>
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

        <div className="flex gap-2">
          <input
            className="flex-1 border rounded px-3 py-2"
            style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
            placeholder="Key name (e.g. pi-mono)"
            value={newKeyName}
            onChange={(e) => setNewKeyName(e.target.value)}
          />
          <button
            onClick={handleCreate}
            disabled={loading || !walletId || !newKeyName.trim()}
            className="px-4 py-2 rounded disabled:opacity-50"
            style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}
          >
            Generate Key
          </button>
        </div>

        {createdToken && (
          <div className="border rounded p-3" style={{ borderColor: "var(--border)", backgroundColor: "var(--card)" }}>
            <p className="text-sm font-medium mb-1">Your new API key (copy it now — we won&apos;t show it again)</p>
            <div className="flex items-center gap-2">
              <code className="text-sm break-all flex-1">{createdToken}</code>
              <button
                onClick={() => copy(createdToken)}
                className="text-xs px-2 py-1 rounded border"
                style={{ borderColor: "var(--border)" }}
              >
                {copied ? "Copied" : "Copy"}
              </button>
            </div>
          </div>
        )}

        {error && <div className="text-sm" style={{ color: "#B45309" }}>{error}</div>}

        <div className="border rounded p-4" style={{ borderColor: "var(--border)", backgroundColor: "var(--card)" }}>
          <h2 className="font-semibold mb-2">Supported MPP chains</h2>
          <p className="text-sm mb-2" style={{ color: "var(--muted-foreground)" }}>
            Gradience routes MPP payments to the cheapest available chain automatically.
          </p>
          <div className="flex flex-wrap gap-2 mb-2">
            {["Tempo", "Base", "BSC (BNB)", "Conflux eSpace", "Conflux Core", "XLayer (OKX)", "Arbitrum", "Polygon", "Optimism", "Solana", "TON"].map((chain) => (
              <span
                key={chain}
                className="text-xs px-2 py-1 rounded"
                style={{ backgroundColor: "var(--muted)", color: "var(--foreground)" }}
              >
                {chain}
              </span>
            ))}
          </div>
        </div>

        <div className="border rounded p-4" style={{ borderColor: "var(--border)", backgroundColor: "var(--card)" }}>
          <h2 className="font-semibold mb-2">Agent configuration</h2>
          <p className="text-sm mb-2" style={{ color: "var(--muted-foreground)" }}>
            Set these environment variables in your agent or IDE:
          </p>
          <pre
            className="text-sm p-3 rounded overflow-auto"
            style={{ backgroundColor: "var(--background)", color: "var(--foreground)" }}
          >
            {`export OPENAI_API_KEY="${createdToken ?? "<your gradience key>"}"
export OPENAI_BASE_URL="${baseUrl}"`}
          </pre>
        </div>

        <div>
          <h2 className="font-semibold mb-2">Active proxy keys</h2>
          {keys.length === 0 ? (
            <p className="text-sm" style={{ color: "var(--muted-foreground)" }}>No proxy keys yet.</p>
          ) : (
            <div className="space-y-2">
              {keys.map((k) => (
                <div
                  key={k.id}
                  className="border rounded p-3 flex items-center justify-between"
                  style={{ borderColor: "var(--border)", backgroundColor: "var(--card)" }}
                >
                  <div>
                    <p className="font-medium text-sm">{k.name}</p>
                    <p className="text-xs" style={{ color: "var(--muted-foreground)" }}>
                      {k.id} · Created {new Date(k.created_at).toLocaleDateString()}
                      {k.expires_at ? ` · Expires ${new Date(k.expires_at).toLocaleDateString()}` : ""}
                    </p>
                  </div>
                  <button
                    onClick={() => handleDelete(k.id)}
                    disabled={loading}
                    className="text-sm px-3 py-1 rounded border disabled:opacity-50"
                    style={{ borderColor: "var(--border)" }}
                  >
                    Revoke
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
