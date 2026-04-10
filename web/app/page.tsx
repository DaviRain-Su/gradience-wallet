"use client";

import Link from "next/link";
import { useEffect } from "react";

const features = [
  {
    title: "OWS Native Vault",
    desc: "Local mnemonic generation, encrypted storage, and multi-chain signing via ows-lib.",
  },
  {
    title: "Policy Engine",
    desc: "Spend limits, intent analysis, dynamic risk signals, time windows, and chain/contract whitelists.",
  },
  {
    title: "DEX Integration",
    desc: "Real 1inch Swap API + Uniswap V3 fallback, executable via Web, CLI, or MCP.",
  },
  {
    title: "MCP Server",
    desc: "JSON-RPC MCP exposing sign_transaction, swap, pay, llm_generate, and more.",
  },
  {
    title: "AI Gateway",
    desc: "Real Anthropic API integration with prepaid balance, cost tracking, and model whitelist.",
  },
  {
    title: "Shared Budget",
    desc: "Workspace-level team budgets with cross-wallet spending tracking and policy enforcement.",
  },
];

const sdks = ["Python", "TypeScript", "Go", "Java", "Ruby"];

export default function LandingPage() {
  useEffect(() => {
    if (
      typeof window !== "undefined" &&
      (window.location.hostname === "localhost" || window.location.hostname === "127.0.0.1")
    ) {
      window.location.href = "/login";
    }
  }, []);

  return (
    <div className="min-h-screen" style={{ backgroundColor: "var(--background)", color: "var(--foreground)" }}>
      {/* Hero */}
      <section className="relative px-6 pt-24 pb-16 text-center">
        <div className="max-w-3xl mx-auto">
          <h1 className="text-5xl md:text-6xl font-extrabold tracking-tight mb-6">
            Agent Wallet
            <br />
            <span style={{ color: "var(--primary)" }}>Orchestration Platform</span>
          </h1>
          <p className="text-lg md:text-xl mb-8" style={{ color: "var(--muted-foreground)" }}>
            Passkey-backed identities. Local multi-chain vaults. Fine-grained policy-gated access for AI agents via MCP.
          </p>
          <div className="flex justify-center gap-4">
            <a
              href="#download"
              className="inline-flex items-center justify-center rounded-lg px-6 py-3 font-semibold transition"
              style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}
            >
              Get Started
            </a>
            <a
              href="https://github.com/open-wallet-standard/core"
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center justify-center rounded-lg px-6 py-3 font-semibold border transition"
              style={{ borderColor: "var(--border)", backgroundColor: "var(--muted)", color: "var(--foreground)" }}
            >
              Learn about OWS
            </a>
          </div>
        </div>
      </section>

      {/* How it works */}
      <section className="px-6 py-16" style={{ backgroundColor: "var(--muted)" }}>
        <div className="max-w-5xl mx-auto">
          <h2 className="text-3xl font-bold text-center mb-10">How it works</h2>
          <div className="grid gap-8 md:grid-cols-3 text-center">
            <div>
              <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full text-lg font-bold" style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}>1</div>
              <h3 className="font-semibold mb-1">Create Identity</h3>
              <p className="text-sm" style={{ color: "var(--muted-foreground)" }}>Register with Passkey and set your local vault passphrase.</p>
            </div>
            <div>
              <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full text-lg font-bold" style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}>2</div>
              <h3 className="font-semibold mb-1">Set Policies</h3>
              <p className="text-sm" style={{ color: "var(--muted-foreground)" }}>Define spend limits, time windows, and whitelists for your agents.</p>
            </div>
            <div>
              <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full text-lg font-bold" style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}>3</div>
              <h3 className="font-semibold mb-1">Connect Agents</h3>
              <p className="text-sm" style={{ color: "var(--muted-foreground)" }}>Use MCP or SDKs to let AI agents act within your policy guardrails.</p>
            </div>
          </div>
        </div>
      </section>

      {/* Download */}
      <section id="download" className="px-6 py-16" style={{ backgroundColor: "var(--card)" }}>
        <div className="max-w-3xl mx-auto text-center">
          <h2 className="text-3xl font-bold mb-4">Run it locally</h2>
          <p className="mb-8" style={{ color: "var(--muted-foreground)" }}>
            Gradience is a local-first wallet. Download the binary for your platform and start your own vault.
          </p>
          <div className="rounded-xl border p-6 text-left font-mono text-sm" style={{ backgroundColor: "var(--muted)", borderColor: "var(--border)" }}>
            <p className="mb-2"># macOS (Apple Silicon)</p>
            <p className="mb-2">curl -L -o gradience.tar.gz https://github.com/DaviRain-Su/gradience-wallet/releases/latest/download/gradience-aarch64-apple-darwin.tar.gz</p>
            <p className="mb-2">tar xzf gradience.tar.gz</p>
            <p className="mb-4">./gradience</p>
            <p className="mb-2"># Linux (x86_64)</p>
            <p className="mb-2">curl -L -o gradience.tar.gz https://github.com/DaviRain-Su/gradience-wallet/releases/latest/download/gradience-x86_64-unknown-linux-gnu.tar.gz</p>
            <p className="mb-2">tar xzf gradience.tar.gz</p>
            <p className="mb-4">./gradience</p>
            <p className="mb-2"># Intel Mac / Windows / other</p>
            <p>cargo install --path crates/gradience-cli --bin gradience</p>
          </div>
          <p className="mt-6 text-sm" style={{ color: "var(--muted-foreground)" }}>
            The command starts a local server and opens your browser automatically.
          </p>
        </div>
      </section>

      {/* Features */}
      <section className="px-6 py-16">
        <div className="max-w-5xl mx-auto">
          <h2 className="text-3xl font-bold text-center mb-10">Core Features</h2>
          <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
            {features.map((f) => (
              <div
                key={f.title}
                className="rounded-xl border p-6 transition"
                style={{ backgroundColor: "var(--muted)", borderColor: "var(--border)" }}
              >
                <h3 className="font-semibold mb-2">{f.title}</h3>
                <p className="text-sm" style={{ color: "var(--muted-foreground)" }}>
                  {f.desc}
                </p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* SDKs */}
      <section className="px-6 py-16" style={{ backgroundColor: "var(--background)" }}>
        <div className="max-w-4xl mx-auto text-center">
          <h2 className="text-3xl font-bold mb-4">Multi-Language SDKs</h2>
          <p className="mb-8" style={{ color: "var(--muted-foreground)" }}>
            Build on top of Gradience with idiomatic SDKs in your favorite language.
          </p>
          <div className="flex flex-wrap justify-center gap-3">
            {sdks.map((sdk) => (
              <span
                key={sdk}
                className="rounded-full px-4 py-1 text-sm font-medium border"
                style={{ borderColor: "var(--border)", backgroundColor: "var(--card)" }}
              >
                {sdk}
              </span>
            ))}
          </div>
          <div className="mt-8">
            <Link
              href="/docs/06-sdk-guide.md"
              className="text-sm underline"
              style={{ color: "var(--primary)" }}
            >
              Read the SDK Guide →
            </Link>
          </div>
        </div>
      </section>

      {/* CTA */}
      <section className="px-6 py-16 text-center" style={{ backgroundColor: "var(--muted)" }}>
        <div className="max-w-2xl mx-auto">
          <h2 className="text-3xl font-bold mb-4">Ready to get started?</h2>
          <p className="mb-8" style={{ color: "var(--muted-foreground)" }}>
            Download the binary, run it locally, and start managing your agent wallets in minutes.
          </p>
          <a
            href="#download"
            className="inline-flex items-center justify-center rounded-lg px-8 py-3 font-semibold transition"
            style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}
          >
            Download & Run
          </a>
        </div>
      </section>

      {/* Footer */}
      <footer className="px-6 py-8 text-center text-sm" style={{ color: "var(--muted-foreground)" }}>
        <p>
          Built for the  ·{" "}
          <a href="https://github.com/open-wallet-standard/core" target="_blank" rel="noreferrer" className="underline">
            OWS
          </a>
        </p>
      </footer>
    </div>
  );
}
