"use client";

import { useState } from "react";
import { apiGet, apiPost } from "@/lib/api";

export default function AiGateway() {
  const [walletId, setWalletId] = useState("");
  const [amount, setAmount] = useState("1000000");
  const [prompt, setPrompt] = useState("Summarize blockchain wallet security in one sentence.");
  const [result, setResult] = useState<any>(null);
  const [msg, setMsg] = useState("");

  async function handleTopup() {
    try {
      await apiPost("/api/ai/topup", { wallet_id: walletId, token: "USDC", amount_raw: amount });
      setMsg("Topup successful");
    } catch (e: any) {
      setMsg(`Topup failed: ${e.message}`);
    }
  }

  async function handleBalance() {
    try {
      const res = await apiGet(`/api/ai/balance/${walletId}?token=USDC`);
      const data = await res.json();
      setResult({ type: "balance", data });
    } catch (e: any) {
      setMsg(`Balance failed: ${e.message}`);
    }
  }

  async function handleGenerate() {
    try {
      const res = await apiPost("/api/ai/generate", {
        wallet_id: walletId,
        provider: "anthropic",
        model: "claude-3-5-sonnet",
        prompt,
      });
      const data = await res.json();
      setResult({ type: "generate", data });
    } catch (e: any) {
      setMsg(`Generate failed: ${e.message}`);
    }
  }

  return (
    <div className="min-h-screen p-8 max-w-2xl mx-auto">
      <h1 className="text-2xl font-bold mb-6">AI Gateway</h1>

      <div className="flex flex-col gap-4 mb-6">
        <input
          className="border rounded px-3 py-2"
          placeholder="Wallet ID"
          value={walletId}
          onChange={(e) => setWalletId(e.target.value)}
        />
        <input
          className="border rounded px-3 py-2"
          placeholder="Amount raw (1 USDC = 1_000_000)"
          value={amount}
          onChange={(e) => setAmount(e.target.value)}
        />
        <div className="flex gap-2">
          <button onClick={handleTopup} className="bg-black text-white px-4 py-2 rounded">Topup</button>
          <button onClick={handleBalance} className="border px-4 py-2 rounded hover:bg-gray-100">Balance</button>
        </div>
      </div>

      <div className="mb-6">
        <textarea
          className="border rounded px-3 py-2 w-full h-24"
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
        />
        <button onClick={handleGenerate} className="mt-2 bg-black text-white px-4 py-2 rounded">Generate</button>
      </div>

      {msg && <p className="text-red-500 text-sm mb-4">{msg}</p>}

      {result?.type === "balance" && (
        <div className="border rounded p-4">
          <p className="font-medium">Balance (raw)</p>
          <p className="font-mono">{result.data.balance_raw}</p>
        </div>
      )}

      {result?.type === "generate" && (
        <div className="border rounded p-4 space-y-2">
          <p className="font-medium">Response</p>
          <p className="text-sm">{result.data.content}</p>
          <p className="text-xs text-gray-500">
            Tokens: {result.data.input_tokens} in / {result.data.output_tokens} out | Cost: {result.data.cost_raw} | Status: {result.data.status}
          </p>
        </div>
      )}
    </div>
  );
}
