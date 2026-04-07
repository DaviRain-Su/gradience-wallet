"use client";

import { useEffect, useState } from "react";
import { apiGet, apiPost } from "@/lib/api";

interface Wallet {
  id: string;
  name: string;
  owner_id: string;
  workspace_id: string | null;
  status: string;
  created_at: string;
  updated_at: string;
}

export default function Dashboard() {
  const [wallets, setWallets] = useState<Wallet[]>([]);
  const [name, setName] = useState("");
  const [loading, setLoading] = useState(false);
  const [msg, setMsg] = useState("");

  async function fetchWallets() {
    try {
      const res = await apiGet("/api/wallets");
      const data = await res.json();
      setWallets(data);
    } catch (e: any) {
      setMsg(`Failed to load wallets: ${e.message}`);
    }
  }

  useEffect(() => {
    fetchWallets();
  }, []);

  async function handleCreate() {
    if (!name.trim()) return;
    setLoading(true);
    try {
      await apiPost("/api/wallets", { name });
      setName("");
      await fetchWallets();
    } catch (e: any) {
      setMsg(`Create failed: ${e.message}`);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="min-h-screen p-8 max-w-3xl mx-auto">
      <h1 className="text-2xl font-bold mb-6">Wallet Dashboard</h1>

      <div className="flex gap-2 mb-6">
        <input
          className="border rounded px-3 py-2 flex-1"
          placeholder="Wallet name"
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
        <button
          onClick={handleCreate}
          disabled={loading}
          className="bg-black text-white px-4 py-2 rounded disabled:opacity-50"
        >
          {loading ? "Creating..." : "Create Wallet"}
        </button>
      </div>

      {msg && <p className="text-red-500 text-sm mb-4">{msg}</p>}

      <div className="space-y-4">
        {wallets.length === 0 && <p className="text-gray-500">No wallets yet.</p>}
        {wallets.map((w) => (
          <div key={w.id} className="border rounded p-4 flex justify-between items-center">
            <div>
              <p className="font-semibold">{w.name}</p>
              <p className="text-sm text-gray-500">{w.id}</p>
            </div>
            <span className="text-xs bg-gray-100 px-2 py-1 rounded">{w.status}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
