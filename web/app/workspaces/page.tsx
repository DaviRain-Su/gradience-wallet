"use client";

import { useEffect, useState } from "react";
import { apiGet, apiPost } from "@/lib/api";

interface Workspace {
  id: string;
  name: string;
  owner_id: string;
  plan: string;
  created_at: string;
}

interface Member {
  workspace_id: string;
  user_id: string;
  role: string;
  invited_at: string;
}

export default function WorkspacesPage() {
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [name, setName] = useState("");
  const [msg, setMsg] = useState("");
  const [selected, setSelected] = useState<string | null>(null);
  const [members, setMembers] = useState<Member[]>([]);
  const [email, setEmail] = useState("");
  const [role, setRole] = useState("member");

  async function fetchWorkspaces() {
    try {
      const res = await apiGet("/api/workspaces");
      const data = await res.json();
      setWorkspaces(data);
    } catch (e: unknown) {
      setMsg(`Failed to load: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  useEffect(() => {
    fetchWorkspaces();
  }, []);

  async function handleCreate() {
    if (!name.trim()) return;
    try {
      await apiPost("/api/workspaces", { name });
      setName("");
      await fetchWorkspaces();
    } catch (e: unknown) {
      setMsg(`Create failed: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  async function loadMembers(wsId: string) {
    setSelected(wsId);
    try {
      const res = await apiGet(`/api/workspaces/${wsId}/members`);
      const data = await res.json();
      setMembers(data);
    } catch (e: unknown) {
      setMsg(`Members failed: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  async function invite(wsId: string) {
    if (!email.trim()) return;
    try {
      await apiPost(`/api/workspaces/${wsId}/members`, { email, role });
      setEmail("");
      await loadMembers(wsId);
    } catch (e: unknown) {
      setMsg(`Invite failed: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  return (
    <div className="min-h-screen p-8 max-w-3xl mx-auto">
      <h1 className="text-2xl font-bold mb-6">Workspaces</h1>

      <div className="flex gap-2 mb-6">
        <input
          className="border rounded px-3 py-2 flex-1"
          placeholder="Workspace name"
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
        <button
          onClick={handleCreate}
          className="bg-black text-white px-4 py-2 rounded"
        >
          Create
        </button>
      </div>

      {msg && <p className="text-red-500 text-sm mb-4">{msg}</p>}

      <div className="space-y-4">
        {workspaces.length === 0 && (
          <p className="text-gray-500">No workspaces yet.</p>
        )}
        {workspaces.map((ws) => (
          <div key={ws.id} className="border rounded p-4">
            <div className="flex justify-between items-center">
              <div>
                <p className="font-semibold">{ws.name}</p>
                <p className="text-xs text-gray-500 font-mono">{ws.id}</p>
                <p className="text-xs text-gray-400">Plan: {ws.plan}</p>
              </div>
              <button
                onClick={() => loadMembers(ws.id)}
                className="text-sm border px-3 py-1 rounded hover:bg-gray-100"
              >
                Members
              </button>
            </div>

            {selected === ws.id && (
              <div className="mt-4 border-t pt-3">
                <p className="text-sm font-medium mb-2">Members</p>
                {members.length === 0 && (
                  <p className="text-xs text-gray-400">No members.</p>
                )}
                <ul className="text-sm space-y-1 mb-3">
                  {members.map((m) => (
                    <li key={m.user_id} className="flex justify-between">
                      <span className="font-mono text-xs">{m.user_id}</span>
                      <span className="text-gray-500 text-xs">{m.role}</span>
                    </li>
                  ))}
                </ul>
                <div className="flex gap-2">
                  <input
                    className="border rounded px-2 py-1 flex-1 text-sm"
                    placeholder="Email"
                    value={email}
                    onChange={(e) => setEmail(e.target.value)}
                  />
                  <select
                    className="border rounded px-2 py-1 text-sm"
                    value={role}
                    onChange={(e) => setRole(e.target.value)}
                  >
                    <option value="member">member</option>
                    <option value="admin">admin</option>
                    <option value="viewer">viewer</option>
                  </select>
                  <button
                    onClick={() => invite(ws.id)}
                    className="bg-black text-white px-3 py-1 rounded text-sm"
                  >
                    Invite
                  </button>
                </div>
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
