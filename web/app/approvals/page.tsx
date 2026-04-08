"use client";

import { useEffect, useState } from "react";
import { apiGet, apiPost } from "@/lib/api";

interface Approval {
  id: string;
  policy_id: string;
  wallet_id: string;
  status: string;
  request_json: string;
  created_at: string;
}

export default function ApprovalsPage() {
  const [approvals, setApprovals] = useState<Approval[]>([]);
  const [filter, setFilter] = useState<"pending" | "history">("pending");
  const [msg, setMsg] = useState("");
  const [loadingId, setLoadingId] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);

  async function fetchApprovals() {
    setRefreshing(true);
    try {
      const res = await apiGet("/api/policy-approvals");
      const data = await res.json();
      setApprovals(data);
      setMsg("");
    } catch (e: unknown) {
      setMsg(`Failed to load: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setRefreshing(false);
    }
  }

  useEffect(() => {
    fetchApprovals();
  }, []);

  async function handleAction(id: string, action: "approve" | "reject") {
    setLoadingId(id + action);
    try {
      await apiPost(`/api/policy-approvals/${id}/${action}`, {});
      setMsg(`${action === "approve" ? "Approved" : "Rejected"} ${id.slice(0, 8)}`);
      await fetchApprovals();
    } catch (e: unknown) {
      setMsg(`Action failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setLoadingId(null);
    }
  }

  const filtered = approvals.filter((a) =>
    filter === "pending" ? a.status === "pending" : a.status !== "pending"
  );

  const pendingCount = approvals.filter((a) => a.status === "pending").length;

  return (
    <div className="min-h-screen p-8 max-w-4xl mx-auto" style={{ backgroundColor: "var(--background)", color: "var(--foreground)" }}>
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold">Approvals</h1>
          <p className="text-sm mt-1" style={{ color: "var(--muted-foreground)" }}>
            Review and manage agent action requests
          </p>
        </div>
        <button
          onClick={fetchApprovals}
          disabled={refreshing}
          className="text-sm px-4 py-2 rounded border disabled:opacity-50"
          style={{ borderColor: "var(--border)", backgroundColor: "var(--muted)", color: "var(--foreground)" }}
        >
          {refreshing ? "Refreshing..." : "Refresh"}
        </button>
      </div>

      <div className="flex gap-2 mb-6">
        <button
          onClick={() => setFilter("pending")}
          className="px-4 py-2 rounded text-sm font-medium transition-colors"
          style={{
            backgroundColor: filter === "pending" ? "var(--primary)" : "var(--muted)",
            color: filter === "pending" ? "var(--primary-foreground)" : "var(--foreground)",
          }}
        >
          Pending ({pendingCount})
        </button>
        <button
          onClick={() => setFilter("history")}
          className="px-4 py-2 rounded text-sm font-medium transition-colors"
          style={{
            backgroundColor: filter === "history" ? "var(--primary)" : "var(--muted)",
            color: filter === "history" ? "var(--primary-foreground)" : "var(--foreground)",
          }}
        >
          History
        </button>
      </div>

      {msg && (
        <p
          className="text-sm mb-4 px-4 py-2 rounded"
          style={{
            backgroundColor: "var(--muted)",
            color: (msg.startsWith("Failed") || msg.startsWith("Action failed")) ? "#B45309" : "#15803D",
          }}
        >
          {msg}
        </p>
      )}

      <div className="space-y-4">
        {filtered.length === 0 && (
          <div className="text-center py-12 rounded-lg" style={{ backgroundColor: "var(--muted)" }}>
            <p style={{ color: "var(--muted-foreground)" }}>
              {filter === "pending"
                ? "No pending approvals. Your agents are behaving."
                : "No approval history yet."}
            </p>
          </div>
        )}
        {filtered.map((a) => (
          <div
            key={a.id}
            className="border rounded-lg p-5 transition-shadow hover:shadow-sm"
            style={{
              borderColor: "var(--border)",
              backgroundColor: a.status === "pending" ? "var(--card)" : "var(--muted)",
            }}
          >
            <div className="flex flex-col sm:flex-row sm:justify-between sm:items-start gap-4">
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 mb-1">
                  <span className="font-semibold text-sm">
                    Approval {a.id.slice(0, 8)}
                  </span>
                  <StatusBadge status={a.status} />
                </div>
                <p className="text-xs font-mono mb-2" style={{ color: "var(--muted-foreground)" }}>
                  Wallet: {a.wallet_id}
                </p>
                <RequestSummary json={a.request_json} />
                <p className="text-xs mt-2" style={{ color: "var(--muted-foreground)" }}>
                  {new Date(a.created_at).toLocaleString()}
                </p>
              </div>
              {a.status === "pending" && (
                <div className="flex gap-2 shrink-0">
                  <button
                    onClick={() => handleAction(a.id, "approve")}
                    disabled={loadingId === a.id + "approve"}
                    className="text-sm px-4 py-2 rounded disabled:opacity-50"
                    style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}
                  >
                    {loadingId === a.id + "approve" ? "..." : "Approve"}
                  </button>
                  <button
                    onClick={() => handleAction(a.id, "reject")}
                    disabled={loadingId === a.id + "reject"}
                    className="text-sm border px-4 py-2 rounded disabled:opacity-50"
                    style={{ borderColor: "var(--border)", backgroundColor: "var(--muted)", color: "var(--foreground)" }}
                  >
                    {loadingId === a.id + "reject" ? "..." : "Reject"}
                  </button>
                </div>
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function StatusBadge({ status }: { status: string }) {
  const colors: Record<string, { backgroundColor: string; color: string }> = {
    pending: { backgroundColor: "#FEF3C7", color: "#92400E" },
    approved: { backgroundColor: "#DCFCE7", color: "#166534" },
    rejected: { backgroundColor: "#FEE2E2", color: "#991B1B" },
  };
  return (
    <span
      className="text-xs font-medium px-2 py-0.5 rounded-full uppercase tracking-wide"
      style={colors[status] || { backgroundColor: "var(--muted)", color: "var(--muted-foreground)" }}
    >
      {status}
    </span>
  );
}

function RequestSummary({ json }: { json: string }) {
  try {
    const obj = JSON.parse(json);
    const parts: string[] = [];
    if (obj.transaction?.to) {
      parts.push(`To: ${obj.transaction.to.slice(0, 12)}...`);
    }
    if (obj.transaction?.value) {
      const val = BigInt(obj.transaction.value);
      if (val > BigInt(0)) {
        parts.push(`Value: ${(Number(val) / 1e18).toFixed(6)} ETH`);
      }
    }
    if (obj.intent?.operation_type) {
      parts.push(`Operation: ${obj.intent.operation_type}`);
    }
    if (obj.intent?.contract_address) {
      parts.push(`Contract: ${obj.intent.contract_address.slice(0, 12)}...`);
    }
    if (parts.length === 0) {
      parts.push(JSON.stringify(obj).slice(0, 120));
    }
    return (
      <div className="flex flex-wrap gap-2">
        {parts.map((part, i) => (
          <span
            key={i}
            className="text-xs px-2 py-1 rounded"
            style={{ backgroundColor: "var(--muted)", color: "var(--foreground)" }}
          >
            {part}
          </span>
        ))}
      </div>
    );
  } catch {
    return (
      <p className="text-xs font-mono break-all" style={{ color: "var(--muted-foreground)" }}>{json}</p>
    );
  }
}
