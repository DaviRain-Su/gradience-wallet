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
  const [msg, setMsg] = useState("");
  const [loadingId, setLoadingId] = useState<string | null>(null);

  async function fetchApprovals() {
    try {
      const res = await apiGet("/api/policy-approvals");
      const data = await res.json();
      setApprovals(data);
    } catch (e: unknown) {
      setMsg(`Failed to load: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  useEffect(() => {
    fetchApprovals();
  }, []);

  async function handleAction(id: string, action: "approve" | "reject") {
    setLoadingId(id + action);
    try {
      await apiPost(`/api/policy-approvals/${id}/${action}`, {});
      setMsg(`${action === "approve" ? "Approved" : "Rejected"} ${id}`);
      await fetchApprovals();
    } catch (e: unknown) {
      setMsg(`Action failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setLoadingId(null);
    }
  }

  return (
    <div className="min-h-screen p-8 max-w-3xl mx-auto">
      <h1 className="text-2xl font-bold mb-6">Policy Approvals</h1>
      {msg && <p className="text-red-500 text-sm mb-4">{msg}</p>}

      <div className="space-y-4">
        {approvals.length === 0 && (
          <p className="text-gray-500">No pending policy approvals.</p>
        )}
        {approvals.map((a) => (
          <div key={a.id} className="border rounded p-4">
            <div className="flex justify-between items-start">
              <div>
                <p className="font-semibold text-sm">Approval {a.id.slice(0, 8)}</p>
                <p className="text-xs text-gray-500 font-mono">Wallet: {a.wallet_id}</p>
                <p className="text-xs text-gray-400">{a.request_json}</p>
              </div>
              <div className="flex gap-2">
                <button
                  onClick={() => handleAction(a.id, "approve")}
                  disabled={loadingId === a.id + "approve"}
                  className="text-sm bg-black text-white px-3 py-1 rounded disabled:opacity-50"
                >
                  {loadingId === a.id + "approve" ? "..." : "Approve"}
                </button>
                <button
                  onClick={() => handleAction(a.id, "reject")}
                  disabled={loadingId === a.id + "reject"}
                  className="text-sm border px-3 py-1 rounded hover:bg-gray-100 disabled:opacity-50"
                >
                  {loadingId === a.id + "reject" ? "..." : "Reject"}
                </button>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
