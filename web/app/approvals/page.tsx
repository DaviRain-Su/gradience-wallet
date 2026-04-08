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
    <div className="min-h-screen p-8 max-w-4xl mx-auto">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold">Approvals</h1>
          <p className="text-sm text-gray-500 mt-1">
            Review and manage agent action requests
          </p>
        </div>
        <button
          onClick={fetchApprovals}
          disabled={refreshing}
          className="text-sm px-4 py-2 rounded border hover:bg-gray-50 disabled:opacity-50"
        >
          {refreshing ? "Refreshing..." : "Refresh"}
        </button>
      </div>

      <div className="flex gap-2 mb-6">
        <button
          onClick={() => setFilter("pending")}
          className={`px-4 py-2 rounded text-sm font-medium transition-colors ${
            filter === "pending"
              ? "bg-black text-white"
              : "bg-gray-100 text-gray-700 hover:bg-gray-200"
          }`}
        >
          Pending ({pendingCount})
        </button>
        <button
          onClick={() => setFilter("history")}
          className={`px-4 py-2 rounded text-sm font-medium transition-colors ${
            filter === "history"
              ? "bg-black text-white"
              : "bg-gray-100 text-gray-700 hover:bg-gray-200"
          }`}
        >
          History
        </button>
      </div>

      {msg && (
        <p
          className={`text-sm mb-4 px-4 py-2 rounded ${
            msg.startsWith("Failed") || msg.startsWith("Action failed")
              ? "bg-red-50 text-red-600"
              : "bg-green-50 text-green-700"
          }`}
        >
          {msg}
        </p>
      )}

      <div className="space-y-4">
        {filtered.length === 0 && (
          <div className="text-center py-12 bg-gray-50 rounded-lg">
            <p className="text-gray-500">
              {filter === "pending"
                ? "No pending approvals. Your agents are behaving."
                : "No approval history yet."}
            </p>
          </div>
        )}
        {filtered.map((a) => (
          <div
            key={a.id}
            className={`border rounded-lg p-5 transition-shadow hover:shadow-sm ${
              a.status === "pending" ? "bg-white" : "bg-gray-50"
            }`}
          >
            <div className="flex flex-col sm:flex-row sm:justify-between sm:items-start gap-4">
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 mb-1">
                  <span className="font-semibold text-sm">
                    Approval {a.id.slice(0, 8)}
                  </span>
                  <StatusBadge status={a.status} />
                </div>
                <p className="text-xs text-gray-500 font-mono mb-2">
                  Wallet: {a.wallet_id}
                </p>
                <RequestSummary json={a.request_json} />
                <p className="text-xs text-gray-400 mt-2">
                  {new Date(a.created_at).toLocaleString()}
                </p>
              </div>
              {a.status === "pending" && (
                <div className="flex gap-2 shrink-0">
                  <button
                    onClick={() => handleAction(a.id, "approve")}
                    disabled={loadingId === a.id + "approve"}
                    className="text-sm bg-black text-white px-4 py-2 rounded disabled:opacity-50"
                  >
                    {loadingId === a.id + "approve" ? "..." : "Approve"}
                  </button>
                  <button
                    onClick={() => handleAction(a.id, "reject")}
                    disabled={loadingId === a.id + "reject"}
                    className="text-sm border px-4 py-2 rounded hover:bg-gray-100 disabled:opacity-50"
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
  const styles: Record<string, string> = {
    pending: "bg-yellow-100 text-yellow-800",
    approved: "bg-green-100 text-green-800",
    rejected: "bg-red-100 text-red-800",
  };
  return (
    <span
      className={`text-xs font-medium px-2 py-0.5 rounded-full uppercase tracking-wide ${
        styles[status] || "bg-gray-100 text-gray-600"
      }`}
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
            className="text-xs px-2 py-1 rounded bg-gray-100 text-gray-700"
          >
            {part}
          </span>
        ))}
      </div>
    );
  } catch {
    return (
      <p className="text-xs text-gray-400 font-mono break-all">{json}</p>
    );
  }
}
