"use client";

import { useEffect, useState } from "react";
import { apiGet } from "@/lib/api";

interface Wallet {
  id: string;
  name: string;
}

interface Tx {
  id: number;
  action: string;
  decision: string;
  tx_hash: string | null;
  created_at: string;
}

interface Approval {
  id: string;
  status: string;
  wallet_id: string;
  request_json: string;
  created_at: string;
}

type ActivityItem =
  | { type: "tx"; data: Tx; walletName: string }
  | { type: "approval"; data: Approval; walletName: string };

export default function ActivityPage() {
  const [items, setItems] = useState<ActivityItem[]>([]);
  const [msg, setMsg] = useState("");
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    async function fetchAll() {
      setLoading(true);
      try {
        const [walletsRes, approvalsRes] = await Promise.all([
          apiGet("/api/wallets"),
          apiGet("/api/policy-approvals"),
        ]);
        const wallets: Wallet[] = await walletsRes.json();
        const approvals: Approval[] = await approvalsRes.json();
        const walletMap = new Map(wallets.map((w) => [w.id, w.name]));

        const txArrays = await Promise.all(
          wallets.map(async (w) => {
            const res = await apiGet(`/api/wallets/${w.id}/transactions`);
            const txs: Tx[] = await res.json();
            return txs.map((t) => ({ type: "tx" as const, data: t, walletName: w.name }));
          })
        );

        const approvalItems: ActivityItem[] = approvals.map((a) => ({
          type: "approval",
          data: a,
          walletName: walletMap.get(a.wallet_id) || a.wallet_id,
        }));

        const all = [...txArrays.flat(), ...approvalItems];
        all.sort((a, b) => +new Date(b.data.created_at) - +new Date(a.data.created_at));
        setItems(all.slice(0, 50));
      } catch (e: unknown) {
        setMsg(`Failed to load activity: ${e instanceof Error ? e.message : String(e)}`);
      } finally {
        setLoading(false);
      }
    }
    fetchAll();
  }, []);

  return (
    <div className="min-h-screen p-8 max-w-3xl mx-auto" style={{ backgroundColor: "var(--background)", color: "var(--foreground)" }}>
      <h1 className="text-2xl font-bold mb-6">Activity</h1>
      {loading && <p style={{ color: "var(--muted-foreground)" }}>Loading...</p>}
      {msg && <p className="text-sm mb-4" style={{ color: "#B45309" }}>{msg}</p>}

      {items.length === 0 && !loading && (
        <p style={{ color: "var(--muted-foreground)" }}>No activity yet.</p>
      )}

      <ul className="space-y-3">
        {items.map((item, idx) => (
          <li
            key={`${item.type}-${item.data.id}-${idx}`}
            className="border rounded-lg p-4"
            style={{ borderColor: "var(--border)", backgroundColor: "var(--card)" }}
          >
            {item.type === "tx" ? (
              <div className="flex justify-between items-center">
                <div>
                  <p className="text-sm font-medium">{item.data.action}</p>
                  <p className="text-xs" style={{ color: "var(--muted-foreground)" }}>
                    {item.walletName}
                  </p>
                </div>
                <div className="text-right">
                  <p className="text-xs font-mono" style={{ color: "var(--muted-foreground)" }}>
                    {item.data.tx_hash || item.data.decision}
                  </p>
                  <p className="text-xs" style={{ color: "var(--muted-foreground)" }}>
                    {new Date(item.data.created_at).toLocaleString()}
                  </p>
                </div>
              </div>
            ) : (
              <div className="flex justify-between items-center">
                <div>
                  <p className="text-sm font-medium">Approval</p>
                  <p className="text-xs" style={{ color: "var(--muted-foreground)" }}>
                    {item.walletName} • {item.data.status}
                  </p>
                </div>
                <div className="text-right">
                  <p className="text-xs" style={{ color: "var(--muted-foreground)" }}>
                    {new Date(item.data.created_at).toLocaleString()}
                  </p>
                </div>
              </div>
            )}
          </li>
        ))}
      </ul>
    </div>
  );
}
