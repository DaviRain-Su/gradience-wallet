"use client";

import { useState } from "react";
import { MppProvider, mppGenerate } from "@/lib/mpp";

const PROVIDERS: { key: MppProvider; label: string; defaultModel: string }[] = [
  { key: "anthropic", label: "Anthropic", defaultModel: "claude-3-5-sonnet" },
  { key: "openai", label: "OpenAI", defaultModel: "gpt-4o" },
  { key: "openrouter", label: "OpenRouter", defaultModel: "openai/gpt-4o" },
  { key: "gemini", label: "Gemini", defaultModel: "gemini-1.5-pro" },
  { key: "groq", label: "Groq", defaultModel: "llama-3.3-70b" },
  { key: "mistral", label: "Mistral", defaultModel: "mistral-large" },
  { key: "deepseek", label: "DeepSeek", defaultModel: "deepseek-v3" },
];

export default function AiGateway() {
  const [provider, setProvider] = useState<MppProvider>("anthropic");
  const [model, setModel] = useState("claude-3-5-sonnet");
  const [prompt, setPrompt] = useState("");
  const [walletId, setWalletId] = useState("");
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleProviderChange = (key: MppProvider) => {
    setProvider(key);
    const p = PROVIDERS.find((x) => x.key === key);
    if (p) setModel(p.defaultModel);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setResult(null);
    if (!walletId.trim() || !prompt.trim()) {
      setError("Please enter wallet ID and prompt");
      return;
    }
    setLoading(true);
    try {
      const resp = await mppGenerate({
        wallet_id: walletId.trim(),
        provider,
        model,
        prompt: prompt.trim(),
      });
      setResult(JSON.stringify(resp.data, null, 2));
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div
      className="min-h-screen p-6 max-w-3xl mx-auto"
      style={{ backgroundColor: "var(--background)", color: "var(--foreground)" }}
    >
      <h1 className="text-2xl font-bold mb-4">AI Gateway (MPP)</h1>
      <p className="text-sm mb-6" style={{ color: "var(--muted-foreground)" }}>
        Pay per request with on-chain stablecoins via MPP.
      </p>

      <form onSubmit={handleSubmit} className="space-y-4">
        <div>
          <label className="block text-sm font-medium mb-1">Wallet ID</label>
          <input
            className="w-full border rounded px-3 py-2"
            style={{
              backgroundColor: "var(--card)",
              borderColor: "var(--border)",
              color: "var(--foreground)",
            }}
            value={walletId}
            onChange={(e) => setWalletId(e.target.value)}
            placeholder="Paste your wallet ID"
          />
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="block text-sm font-medium mb-1">Provider</label>
            <select
              className="w-full border rounded px-3 py-2"
              style={{
                backgroundColor: "var(--card)",
                borderColor: "var(--border)",
                color: "var(--foreground)",
              }}
              value={provider}
              onChange={(e) => handleProviderChange(e.target.value as MppProvider)}
            >
              {PROVIDERS.map((p) => (
                <option key={p.key} value={p.key}>
                  {p.label}
                </option>
              ))}
            </select>
          </div>
          <div>
            <label className="block text-sm font-medium mb-1">Model</label>
            <input
              className="w-full border rounded px-3 py-2"
              style={{
                backgroundColor: "var(--card)",
                borderColor: "var(--border)",
                color: "var(--foreground)",
              }}
              value={model}
              onChange={(e) => setModel(e.target.value)}
            />
          </div>
        </div>

        <div>
          <label className="block text-sm font-medium mb-1">Prompt</label>
          <textarea
            className="w-full border rounded px-3 py-2 h-32"
            style={{
              backgroundColor: "var(--card)",
              borderColor: "var(--border)",
              color: "var(--foreground)",
            }}
            value={prompt}
            onChange={(e) => setPrompt(e.target.value)}
            placeholder="Type your prompt here..."
          />
        </div>

        <button
          type="submit"
          disabled={loading}
          className="px-4 py-2 rounded disabled:opacity-50"
          style={{
            backgroundColor: "var(--primary)",
            color: "var(--primary-foreground)",
          }}
        >
          {loading ? "Generating..." : "Generate via MPP"}
        </button>
      </form>

      {error && (
        <div className="mt-4 text-sm" style={{ color: "#B45309" }}>
          {error}
        </div>
      )}

      {result && (
        <div
          className="mt-6 border rounded p-4 overflow-auto text-sm"
          style={{ backgroundColor: "var(--card)", borderColor: "var(--border)" }}
        >
          <pre className="whitespace-pre-wrap">{result}</pre>
        </div>
      )}
    </div>
  );
}
