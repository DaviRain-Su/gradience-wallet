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
  const [createLoading, setCreateLoading] = useState(false);
  const [membersLoading, setMembersLoading] = useState(false);
  const [inviteLoading, setInviteLoading] = useState(false);

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
    setCreateLoading(true);
    try {
      await apiPost("/api/workspaces", { name });
      setName("");
      await fetchWorkspaces();
    } catch (e: unknown) {
      setMsg(`Create failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setCreateLoading(false);
    }
  }

  async function loadMembers(wsId: string) {
    setSelected(wsId);
    setMembersLoading(true);
    try {
      const res = await apiGet(`/api/workspaces/${wsId}/members`);
      const data = await res.json();
      setMembers(data);
    } catch (e: unknown) {
      setMsg(`Members failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setMembersLoading(false);
    }
  }

  async function invite(wsId: string) {
    if (!email.trim()) return;
    setInviteLoading(true);
    try {
      await apiPost(`/api/workspaces/${wsId}/members`, { email, role });
      setEmail("");
      await loadMembers(wsId);
    } catch (e: unknown) {
      setMsg(`Invite failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setInviteLoading(false);
    }
  }

  return (
    <div className="min-h-screen p-8 max-w-3xl mx-auto" style={{ backgroundColor: "var(--background)", color: "var(--foreground)" }}>
      <h1 className="text-2xl font-bold mb-6">Workspaces</h1>

      <div className="flex gap-2 mb-6">
        <input
          className="border rounded px-3 py-2 flex-1"
          style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
          placeholder="Workspace name"
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
        <button
          onClick={handleCreate}
          disabled={createLoading}
          className="px-4 py-2 rounded disabled:opacity-50"
          style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}
        >
          {createLoading ? "Creating..." : "Create"}
        </button>
      </div>

      {msg && <p className="text-sm mb-4" style={{ color: "#B45309" }}>{msg}</p>}

      <div className="space-y-4">
        {workspaces.length === 0 && (
          <p style={{ color: "var(--muted-foreground)" }}>No workspaces yet.</p>
        )}
        {workspaces.map((ws) => (
          <div key={ws.id} className="border rounded p-4" style={{ borderColor: "var(--border)", backgroundColor: "var(--card)" }}>
            <div className="flex justify-between items-center">
              <div>
                <p className="font-semibold">{ws.name}</p>
                <p className="text-xs font-mono" style={{ color: "var(--muted-foreground)" }}>{ws.id}</p>
                <p className="text-xs" style={{ color: "var(--muted-foreground)" }}>Plan: {ws.plan}</p>
              </div>
              <button
                onClick={() => loadMembers(ws.id)}
                disabled={membersLoading && selected === ws.id}
                className="text-sm border px-3 py-1 rounded disabled:opacity-50"
                style={{ borderColor: "var(--border)", backgroundColor: "var(--muted)", color: "var(--foreground)" }}
              >
                {membersLoading && selected === ws.id ? "Loading..." : "Members"}
              </button>
            </div>

            {selected === ws.id && (
              <div className="mt-4 border-t pt-3" style={{ borderColor: "var(--border)" }}>
                <p className="text-sm font-medium mb-2">Members</p>
                {members.length === 0 && (
                  <p className="text-xs" style={{ color: "var(--muted-foreground)" }}>No members.</p>
                )}
                <ul className="text-sm space-y-1 mb-3">
                  {members.map((m) => (
                    <li key={m.user_id} className="flex justify-between">
                      <span className="font-mono text-xs" style={{ color: "var(--muted-foreground)" }}>{m.user_id}</span>
                      <span className="text-xs" style={{ color: "var(--muted-foreground)" }}>{m.role}</span>
                    </li>
                  ))}
                </ul>
                <div className="flex gap-2">
                  <input
                    className="border rounded px-2 py-1 flex-1 text-sm"
                    style={{ backgroundColor: "var(--background)", borderColor: "var(--border)", color: "var(--foreground)" }}
                    placeholder="Email"
                    value={email}
                    onChange={(e) => setEmail(e.target.value)}
                  />
                  <select
                    className="border rounded px-2 py-1 text-sm"
                    style={{ backgroundColor: "var(--background)", borderColor: "var(--border)", color: "var(--foreground)" }}
                    value={role}
                    onChange={(e) => setRole(e.target.value)}
                  >
                    <option value="member">member</option>
                    <option value="admin">admin</option>
                    <option value="viewer">viewer</option>
                  </select>
                  <button
                    onClick={() => invite(ws.id)}
                    disabled={inviteLoading}
                    className="px-3 py-1 rounded text-sm disabled:opacity-50"
                    style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}
                  >
                    {inviteLoading ? "Inviting..." : "Invite"}
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
