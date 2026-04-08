"use client";

import { useEffect, useState } from "react";
import { apiGet } from "@/lib/api";

interface Wallet {
  id: string;
  name: string;
  status: string;
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
  created_at: string;
}

interface ActivityItem {
  type: "tx" | "approval";
  id: string;
  wallet_id: string;
  wallet_name: string;
  title: string;
  subtitle: string;
  created_at: string;
  status?: string;
}

export default function AgentsPage() {
  const [wallets, setWallets] = useState<Wallet[]>([]);
  const [pendingApprovals, setPendingApprovals] = useState(0);
  const [activities, setActivities] = useState<ActivityItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [msg, setMsg] = useState("");

  async function fetchAll() {
    setLoading(true);
    try {
      const [walletsRes, approvalsRes] = await Promise.all([
        apiGet("/api/wallets"),
        apiGet("/api/policy-approvals"),
      ]);
      const walletsData: Wallet[] = await walletsRes.json();
      const approvalsData: Approval[] = await approvalsRes.json();

      setWallets(walletsData);
      setPendingApprovals(approvalsData.filter((a) => a.status === "pending").length);

      // Fetch recent transactions for first 3 wallets
      const txPromises = walletsData.slice(0, 3).map(async (w) => {
        try {
          const res = await apiGet(`/api/wallets/${w.id}/transactions`);
          const txs: Tx[] = await res.json();
          return txs.slice(0, 3).map((t) => ({
            type: "tx" as const,
            id: String(t.id),
            wallet_id: w.id,
            wallet_name: w.name,
            title: t.action,
            subtitle: t.tx_hash ? t.tx_hash.slice(0, 14) + "..." : t.decision,
            created_at: t.created_at,
            status: t.decision,
          }));
        } catch {
          return [];
        }
      });

      const txResults = await Promise.all(txPromises);
      const approvalActivities: ActivityItem[] = approvalsData
        .filter((a) => a.status === "pending")
        .slice(0, 3)
        .map((a) => ({
          type: "approval" as const,
          id: a.id,
          wallet_id: "",
          wallet_name: "Policy Engine",
          title: "Approval Request",
          subtitle: `ID ${a.id.slice(0, 8)}`,
          created_at: a.created_at,
          status: "pending",
        }));

      const allActivities = [...txResults.flat(), ...approvalActivities].sort(
        (a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
      );

      setActivities(allActivities.slice(0, 8));
      setMsg("");
    } catch (e: unknown) {
      setMsg(`Failed to load: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    fetchAll();
  }, []);

  return (
    <div className="min-h-screen p-8 max-w-4xl mx-auto" style={{ backgroundColor: "var(--background)", color: "var(--foreground)" }}>
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold">Agent Monitor</h1>
          <p className="text-sm mt-1" style={{ color: "var(--muted-foreground)" }}>
            Real-time overview of agent activity and system health
          </p>
        </div>
        <button
          onClick={fetchAll}
          disabled={loading}
          className="text-sm px-4 py-2 rounded border disabled:opacity-50"
          style={{ borderColor: "var(--border)", backgroundColor: "var(--muted)", color: "var(--foreground)" }}
        >
          {loading ? "Loading..." : "Refresh"}
        </button>
      </div>

      {msg && (
        <p className="text-sm px-4 py-2 rounded mb-6" style={{ backgroundColor: "var(--muted)", color: "#B45309" }}>
          {msg}
        </p>
      )}

      <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 mb-8">
        <MetricCard
          label="Active Wallets"
          value={wallets.length}
          trend="up"
        />
        <MetricCard
          label="Pending Approvals"
          value={pendingApprovals}
          trend={pendingApprovals > 0 ? "warning" : "up"}
        />
        <MetricCard
          label="Recent Activities"
          value={activities.length}
          trend="up"
        />
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div className="border rounded-lg p-5" style={{ backgroundColor: "var(--card)", borderColor: "var(--border)" }}>
          <h2 className="text-lg font-semibold mb-4">Recent Activity</h2>
          {activities.length === 0 && (
            <p className="text-sm" style={{ color: "var(--muted-foreground)" }}>
              No recent activity to display.
            </p>
          )}
          <div className="space-y-3">
            {activities.map((a) => (
              <div
                key={a.id + a.type}
                className="flex items-start gap-3 p-3 rounded-lg transition-colors"
                style={{ backgroundColor: "var(--muted)" }}
              >
                <div
                  className="w-2 h-2 mt-2 rounded-full shrink-0"
                  style={{
                    backgroundColor:
                      a.type === "tx"
                        ? a.status === "allow"
                          ? "#22c55e"
                          : "#ef4444"
                        : "#eab308",
                  }}
                />
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium">{a.title}</p>
                  <p className="text-xs truncate" style={{ color: "var(--muted-foreground)" }}>
                    {a.wallet_name} • {a.subtitle}
                  </p>
                  <p className="text-xs mt-0.5" style={{ color: "var(--muted-foreground)" }}>
                    {new Date(a.created_at).toLocaleString()}
                  </p>
                </div>
              </div>
            ))}
          </div>
        </div>

        <div className="border rounded-lg p-5" style={{ backgroundColor: "var(--card)", borderColor: "var(--border)" }}>
          <h2 className="text-lg font-semibold mb-4">Quick Actions</h2>
          <div className="space-y-3">
            <QuickAction href="/ai" label="AI Gateway" desc="Top up balance or run inference" />
            <QuickAction href="/policies" label="Policies" desc="Manage agent guardrails" />
            <QuickAction href="/approvals" label="Approvals" desc="Review pending requests" />
            <QuickAction href="/dashboard" label="Dashboard" desc="View wallets and assets" />
          </div>
        </div>
      </div>
    </div>
  );
}

function MetricCard({
  label,
  value,
  trend,
}: {
  label: string;
  value: number;
  trend: "up" | "down" | "warning";
}) {
  const colors = {
    up: { text: "#22c55e", icon: "●" },
    down: { text: "#ef4444", icon: "▼" },
    warning: { text: "#eab308", icon: "▲" },
  };
  return (
    <div className="border rounded-lg p-4" style={{ backgroundColor: "var(--card)", borderColor: "var(--border)" }}>
      <p className="text-sm" style={{ color: "var(--muted-foreground)" }}>{label}</p>
      <div className="flex items-baseline gap-2 mt-1">
        <p className="text-2xl font-bold">{value}</p>
        <span className="text-xs font-medium" style={{ color: colors[trend].text }}>
          {colors[trend].icon}
        </span>
      </div>
    </div>
  );
}

function QuickAction({
  href,
  label,
  desc,
}: {
  href: string;
  label: string;
  desc: string;
}) {
  return (
    <a
      href={href}
      className="flex items-center justify-between p-3 rounded-lg border transition-colors"
      style={{ borderColor: "var(--border)", backgroundColor: "var(--muted)", color: "var(--foreground)" }}
    >
      <div>
        <p className="text-sm font-medium">{label}</p>
        <p className="text-xs" style={{ color: "var(--muted-foreground)" }}>{desc}</p>
      </div>
      <span style={{ color: "var(--muted-foreground)" }}>→</span>
    </a>
  );
}
