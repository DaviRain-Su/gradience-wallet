"use client";

import { useEffect, useState } from "react";
import { apiGet, apiPost } from "@/lib/api";

interface Wallet {
  id: string;
  name: string;
}

interface Workspace {
  id: string;
  name: string;
}

interface Policy {
  id: string;
  name: string;
  wallet_id: string | null;
  workspace_id: string | null;
  rules_json: string;
  status: string;
}

export default function PoliciesPage() {
  const [wallets, setWallets] = useState<Wallet[]>([]);
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [walletPolicies, setWalletPolicies] = useState<Record<string, Policy[]>>({});
  const [workspacePolicies, setWorkspacePolicies] = useState<Record<string, Policy[]>>({});

  const [targetType, setTargetType] = useState<"wallet" | "workspace">("wallet");
  const [selectedWallet, setSelectedWallet] = useState<string>("");
  const [selectedWorkspace, setSelectedWorkspace] = useState<string>("");

  const [policyName, setPolicyName] = useState("");
  const [chainIds, setChainIds] = useState("eip155:8453");
  const [spendLimit, setSpendLimit] = useState("0.01");
  const [contracts, setContracts] = useState("");
  const [operations, setOperations] = useState("transfer,swap");
  const [startHour, setStartHour] = useState(0);
  const [endHour, setEndHour] = useState(23);

  const [sharedBudgetEnabled, setSharedBudgetEnabled] = useState(false);
  const [sharedBudgetAmount, setSharedBudgetAmount] = useState("1");
  const [sharedBudgetToken, setSharedBudgetToken] = useState("ETH");
  const [sharedBudgetPeriod, setSharedBudgetPeriod] = useState("monthly");

  const [loading, setLoading] = useState(false);
  const [msg, setMsg] = useState("");

  useEffect(() => {
    fetchWallets();
    fetchWorkspaces();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function fetchWallets() {
    try {
      const res = await apiGet("/api/wallets");
      const data = await res.json();
      setWallets(data);
      if (data.length > 0) {
        setSelectedWallet(data[0].id);
      }
      data.forEach((w: Wallet) => fetchWalletPolicies(w.id));
    } catch (e: unknown) {
      setMsg(`Failed to load wallets: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  async function fetchWorkspaces() {
    try {
      const res = await apiGet("/api/workspaces");
      const data = await res.json();
      setWorkspaces(data);
      if (data.length > 0) {
        setSelectedWorkspace(data[0].id);
      }
      data.forEach((w: Workspace) => fetchWorkspacePolicies(w.id));
    } catch (e: unknown) {
      setMsg(`Failed to load workspaces: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  async function fetchWalletPolicies(walletId: string) {
    try {
      const res = await apiGet(`/api/wallets/${walletId}/policies`);
      const data = await res.json();
      setWalletPolicies((prev) => ({ ...prev, [walletId]: data }));
    } catch {
      // ignore
    }
  }

  async function fetchWorkspacePolicies(workspaceId: string) {
    try {
      const res = await apiGet(`/api/workspaces/${workspaceId}/policies`);
      const data = await res.json();
      setWorkspacePolicies((prev) => ({ ...prev, [workspaceId]: data }));
    } catch {
      // ignore
    }
  }

  async function handleCreate() {
    if (!policyName.trim()) return;
    if (targetType === "wallet" && !selectedWallet) return;
    if (targetType === "workspace" && !selectedWorkspace) return;
    setLoading(true);
    try {
      const rules = [] as Array<Record<string, unknown>>;
      if (targetType === "wallet") {
        const chains = chainIds
          .split(",")
          .map((s) => s.trim())
          .filter(Boolean);
        if (chains.length > 0) {
          rules.push({ type: "chain_whitelist", chain_ids: chains });
        }
        const limitWei = parseFloat(spendLimit) * 1e18;
        rules.push({ type: "spend_limit", max: Math.floor(limitWei).toString(), token: "ETH" });

        const contractList = contracts
          .split("\n")
          .map((s) => s.trim())
          .filter(Boolean);
        if (contractList.length > 0) {
          rules.push({ type: "contract_whitelist", contracts: contractList });
        }

        const opList = operations
          .split(",")
          .map((s) => s.trim())
          .filter(Boolean);
        if (opList.length > 0) {
          rules.push({ type: "operation_type", allowed: opList });
        }

        rules.push({
          type: "time_window",
          start_hour: startHour,
          end_hour: endHour,
          timezone: "UTC",
        });
      }
      if (targetType === "workspace" && sharedBudgetEnabled) {
        const limitWei = parseFloat(sharedBudgetAmount) * 1e18;
        rules.push({
          type: "shared_budget",
          max: Math.floor(limitWei).toString(),
          token: sharedBudgetToken,
          period: sharedBudgetPeriod,
        });
      }

      const content = JSON.stringify({ name: policyName, rules });
      if (targetType === "wallet") {
        await apiPost(`/api/wallets/${selectedWallet}/policies`, { content });
        await fetchWalletPolicies(selectedWallet);
      } else {
        await apiPost(`/api/workspaces/${selectedWorkspace}/policies`, { content });
        await fetchWorkspacePolicies(selectedWorkspace);
      }
      setMsg("Policy created successfully");
      setPolicyName("");
    } catch (e: unknown) {
      setMsg(`Create failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="min-h-screen p-8 max-w-3xl mx-auto" style={{ backgroundColor: "var(--background)", color: "var(--foreground)" }}>
      <h1 className="text-2xl font-bold mb-6">Policy Management</h1>

      <div className="mb-6 border rounded p-4" style={{ backgroundColor: "var(--card)", borderColor: "var(--border)" }}>
        <h2 className="font-semibold mb-4">Create Policy</h2>
        <div className="grid gap-4">
          <div>
            <label className="block text-sm mb-1">Target</label>
            <select
              className="border rounded px-3 py-2 w-full"
              style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
              value={targetType}
              onChange={(e) => setTargetType(e.target.value as "wallet" | "workspace")}
            >
              <option value="wallet">Wallet Policy</option>
              <option value="workspace">Workspace Shared Budget</option>
            </select>
          </div>

          {targetType === "wallet" && (
            <div>
              <label className="block text-sm mb-1">Wallet</label>
              <select
                className="border rounded px-3 py-2 w-full"
                style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
                value={selectedWallet}
                onChange={(e) => setSelectedWallet(e.target.value)}
              >
                {wallets.map((w) => (
                  <option key={w.id} value={w.id}>
                    {w.name}
                  </option>
                ))}
              </select>
            </div>
          )}

          {targetType === "workspace" && (
            <div>
              <label className="block text-sm mb-1">Workspace</label>
              <select
                className="border rounded px-3 py-2 w-full"
                style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
                value={selectedWorkspace}
                onChange={(e) => setSelectedWorkspace(e.target.value)}
              >
                {workspaces.map((w) => (
                  <option key={w.id} value={w.id}>
                    {w.name}
                  </option>
                ))}
              </select>
            </div>
          )}

          <div>
            <label className="block text-sm mb-1">Policy Name</label>
            <input
              className="border rounded px-3 py-2 w-full"
              style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
              placeholder="e.g. Conservative"
              value={policyName}
              onChange={(e) => setPolicyName(e.target.value)}
            />
          </div>

          {targetType === "wallet" && (
            <>
              <div>
                <label className="block text-sm mb-1">Allowed Chains (comma separated)</label>
                <input
                  className="border rounded px-3 py-2 w-full"
                  style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
                  placeholder="eip155:8453, eip155:1"
                  value={chainIds}
                  onChange={(e) => setChainIds(e.target.value)}
                />
              </div>

              <div>
                <label className="block text-sm mb-1">Spend Limit (ETH)</label>
                <input
                  className="border rounded px-3 py-2 w-full"
                  style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
                  type="number"
                  step="0.001"
                  value={spendLimit}
                  onChange={(e) => setSpendLimit(e.target.value)}
                />
              </div>

              <div>
                <label className="block text-sm mb-1">Allowed Contracts (one per line)</label>
                <textarea
                  className="border rounded px-3 py-2 w-full h-20"
                  style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
                  placeholder="0x..."
                  value={contracts}
                  onChange={(e) => setContracts(e.target.value)}
                />
              </div>

              <div>
                <label className="block text-sm mb-1">Allowed Operations (comma separated)</label>
                <input
                  className="border rounded px-3 py-2 w-full"
                  style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
                  placeholder="transfer, swap, stake"
                  value={operations}
                  onChange={(e) => setOperations(e.target.value)}
                />
              </div>

              <div className="flex gap-4">
                <div className="flex-1">
                  <label className="block text-sm mb-1">Start Hour</label>
                  <input
                    className="border rounded px-3 py-2 w-full"
                    style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
                    type="number"
                    min={0}
                    max={23}
                    value={startHour}
                    onChange={(e) => setStartHour(parseInt(e.target.value || "0", 10))}
                  />
                </div>
                <div className="flex-1">
                  <label className="block text-sm mb-1">End Hour</label>
                  <input
                    className="border rounded px-3 py-2 w-full"
                    style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
                    type="number"
                    min={0}
                    max={23}
                    value={endHour}
                    onChange={(e) => setEndHour(parseInt(e.target.value || "23", 10))}
                  />
                </div>
              </div>
            </>
          )}

          {targetType === "workspace" && (
            <>
              <div className="flex items-center gap-2">
                <input
                  id="sb"
                  type="checkbox"
                  checked={sharedBudgetEnabled}
                  onChange={(e) => setSharedBudgetEnabled(e.target.checked)}
                />
                <label htmlFor="sb" className="text-sm">Enable Shared Budget</label>
              </div>
              {sharedBudgetEnabled && (
                <>
                  <div>
                    <label className="block text-sm mb-1">Shared Budget Amount (ETH)</label>
                    <input
                      className="border rounded px-3 py-2 w-full"
                      style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
                      type="number"
                      step="0.1"
                      value={sharedBudgetAmount}
                      onChange={(e) => setSharedBudgetAmount(e.target.value)}
                    />
                  </div>
                  <div>
                    <label className="block text-sm mb-1">Token</label>
                    <input
                      className="border rounded px-3 py-2 w-full"
                      style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
                      value={sharedBudgetToken}
                      onChange={(e) => setSharedBudgetToken(e.target.value)}
                    />
                  </div>
                  <div>
                    <label className="block text-sm mb-1">Period</label>
                    <select
                      className="border rounded px-3 py-2 w-full"
                      style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
                      value={sharedBudgetPeriod}
                      onChange={(e) => setSharedBudgetPeriod(e.target.value)}
                    >
                      <option value="daily">Daily</option>
                      <option value="weekly">Weekly</option>
                      <option value="monthly">Monthly</option>
                    </select>
                  </div>
                </>
              )}
            </>
          )}

          <button
            onClick={handleCreate}
            disabled={loading}
            className="px-4 py-2 rounded disabled:opacity-50"
            style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}
          >
            {loading ? "Creating..." : "Create Policy"}
          </button>
        </div>
      </div>

      {msg && <p className="text-sm mb-4" style={{ color: "#B45309" }}>{msg}</p>}

      <div className="space-y-6">
        {wallets.map((w) => {
          const list = walletPolicies[w.id] || [];
          if (list.length === 0) return null;
          return (
            <div key={w.id} className="border rounded p-4" style={{ backgroundColor: "var(--card)", borderColor: "var(--border)" }}>
              <p className="font-semibold">{w.name} (Wallet)</p>
              <div className="mt-2 space-y-2">
                {list.map((p) => (
                  <div key={p.id} className="text-sm rounded px-3 py-2" style={{ backgroundColor: "var(--muted)" }}>
                    <p className="font-medium">{p.name}</p>
                    <p className="text-xs" style={{ color: "var(--muted-foreground)" }}>
                      {formatPolicySummary(p.rules_json)}
                    </p>
                  </div>
                ))}
              </div>
            </div>
          );
        })}
        {workspaces.map((w) => {
          const list = workspacePolicies[w.id] || [];
          if (list.length === 0) return null;
          return (
            <div key={w.id} className="border rounded p-4" style={{ backgroundColor: "var(--card)", borderColor: "var(--border)" }}>
              <p className="font-semibold">{w.name} (Workspace)</p>
              <div className="mt-2 space-y-2">
                {list.map((p) => (
                  <div key={p.id} className="text-sm rounded px-3 py-2" style={{ backgroundColor: "var(--muted)" }}>
                    <p className="font-medium">{p.name}</p>
                    <p className="text-xs" style={{ color: "var(--muted-foreground)" }}>
                      {formatPolicySummary(p.rules_json)}
                    </p>
                  </div>
                ))}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function formatPolicySummary(rulesJson: string): string {
  try {
    const obj = JSON.parse(rulesJson);
    const rules = obj.rules || [];
    const parts: string[] = [];
    for (const r of rules) {
      if (r.type === "chain_whitelist" && r.chain_ids?.length) {
        parts.push(`Chains: ${r.chain_ids.join(", ")}`);
      }
      if (r.type === "spend_limit") {
        const eth = (parseInt(r.max, 10) / 1e18).toFixed(4);
        parts.push(`Limit: ${eth} ${r.token || "ETH"}`);
      }
      if (r.type === "contract_whitelist" && r.contracts?.length) {
        parts.push(`Contracts: ${r.contracts.length}`);
      }
      if (r.type === "operation_type" && r.allowed?.length) {
        parts.push(`Ops: ${r.allowed.join(", ")}`);
      }
      if (r.type === "time_window") {
        parts.push(`Window: ${r.start_hour}-${r.end_hour} ${r.timezone}`);
      }
      if (r.type === "shared_budget") {
        const eth = (parseInt(r.max, 10) / 1e18).toFixed(4);
        parts.push(`Shared: ${eth} ${r.token || "ETH"}/${r.period}`);
      }
    }
    return parts.join(" | ") || "No rules";
  } catch {
    return "Invalid policy";
  }
}
