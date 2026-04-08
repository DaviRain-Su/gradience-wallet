"use client";

import { useState } from "react";
import { apiGet, apiPost } from "@/lib/api";

export default function AiGateway() {
  const [walletId, setWalletId] = useState("");
  const [amount, setAmount] = useState("1000000");
  const [prompt, setPrompt] = useState("Summarize blockchain wallet security in one sentence.");
  const [result, setResult] = useState<{ type: string; data: unknown } | null>(null);
  const [msg, setMsg] = useState("");
  const [topupLoading, setTopupLoading] = useState(false);
  const [balanceLoading, setBalanceLoading] = useState(false);
  const [generateLoading, setGenerateLoading] = useState(false);

  async function handleTopup() {
    setTopupLoading(true);
    try {
      await apiPost("/api/ai/topup", { wallet_id: walletId, token: "USDC", amount_raw: amount });
      setMsg("Topup successful");
    } catch (e: unknown) {
      setMsg(`Topup failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setTopupLoading(false);
    }
  }

  async function handleBalance() {
    setBalanceLoading(true);
    try {
      const res = await apiGet(`/api/ai/balance/${walletId}?token=USDC`);
      const data = await res.json();
      setResult({ type: "balance", data });
    } catch (e: unknown) {
      setMsg(`Balance failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setBalanceLoading(false);
    }
  }

  async function handleGenerate() {
    setGenerateLoading(true);
    try {
      const res = await apiPost("/api/ai/generate", {
        wallet_id: walletId,
        provider: "anthropic",
        model: "claude-3-5-sonnet",
        prompt,
      });
      const data = await res.json();
      setResult({ type: "generate", data });
    } catch (e: unknown) {
      setMsg(`Generate failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setGenerateLoading(false);
    }
  }

  return (
    <div className="min-h-screen p-8 max-w-2xl mx-auto" style={{ backgroundColor: "var(--background)", color: "var(--foreground)" }}>
      <h1 className="text-2xl font-bold mb-6">AI Gateway</h1>

      <div className="flex flex-col gap-4 mb-6">
        <input
          className="border rounded px-3 py-2"
          style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
          placeholder="Wallet ID"
          value={walletId}
          onChange={(e) => setWalletId(e.target.value)}
        />
        <input
          className="border rounded px-3 py-2"
          style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
          placeholder="Amount raw (1 USDC = 1_000_000)"
          value={amount}
          onChange={(e) => setAmount(e.target.value)}
        />
        <div className="flex gap-2">
          <button onClick={handleTopup} disabled={topupLoading} className="px-4 py-2 rounded disabled:opacity-50" style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}>{topupLoading ? "Loading..." : "Topup"}</button>
          <button onClick={handleBalance} disabled={balanceLoading} className="border px-4 py-2 rounded disabled:opacity-50" style={{ borderColor: "var(--border)", backgroundColor: "var(--muted)", color: "var(--foreground)" }}>{balanceLoading ? "Loading..." : "Balance"}</button>
        </div>
      </div>

      <div className="mb-6">
        <textarea
          className="border rounded px-3 py-2 w-full h-24"
          style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
        />
        <button onClick={handleGenerate} disabled={generateLoading} className="mt-2 px-4 py-2 rounded disabled:opacity-50" style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}>{generateLoading ? "Generating..." : "Generate"}</button>
      </div>

      {msg && <p className="text-sm mb-4" style={{ color: "#B45309" }}>{msg}</p>}

      {result?.type === "balance" && (
        <div className="border rounded p-4" style={{ borderColor: "var(--border)", backgroundColor: "var(--card)" }}>
          <p className="font-medium">Balance (raw)</p>
          <p className="font-mono">{(result.data as { balance_raw?: string }).balance_raw}</p>
        </div>
      )}

      {result?.type === "generate" && (
        <div className="border rounded p-4 space-y-2" style={{ borderColor: "var(--border)", backgroundColor: "var(--card)" }}>
          <p className="font-medium">Response</p>
          <p className="text-sm">{(result.data as { content?: string }).content}</p>
          <p className="text-xs" style={{ color: "var(--muted-foreground)" }}>
            Tokens: {(result.data as { input_tokens?: number }).input_tokens} in / {(result.data as { output_tokens?: number }).output_tokens} out | Cost: {(result.data as { cost_raw?: string }).cost_raw} | Status: {(result.data as { status?: string }).status}
          </p>
        </div>
      )}
    </div>
  );
}
