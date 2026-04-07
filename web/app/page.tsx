"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { registerPasskey, loginPasskey, unlockVault } from "@/lib/webauthn";
import { apiPost } from "@/lib/api";

export default function Home() {
  const [username, setUsername] = useState("");
  const [email, setEmail] = useState("");
  const [passphrase, setPassphrase] = useState("");
  const [step, setStep] = useState<"login" | "forgot" | "unlock">("login");
  const [recoveryCode, setRecoveryCode] = useState("");
  const [msg, setMsg] = useState("");
  const router = useRouter();

  async function handleRegister() {
    try {
      await registerPasskey(username, passphrase, email);
      router.push("/dashboard");
    } catch (e: unknown) {
      setMsg(`Register failed: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  async function handleLogin() {
    try {
      await loginPasskey(username);
      setStep("unlock");
      setMsg("");
    } catch (e: unknown) {
      setMsg(`Login failed: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  async function handleUnlock() {
    try {
      await unlockVault(passphrase);
      router.push("/dashboard");
    } catch (e: unknown) {
      setMsg(`Unlock failed: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  async function handleSendRecovery() {
    try {
      await apiPost("/api/auth/recover/initiate", { username });
      setMsg("Recovery code sent (check API console output for mock email).");
    } catch (e: unknown) {
      setMsg(`Send failed: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  async function handleVerifyRecovery() {
    try {
      const res = await apiPost("/api/auth/recover/verify", { username, code: recoveryCode });
      const data = await res.json();
      localStorage.setItem("gradience_token", data.token);
      setStep("unlock");
      setMsg("Recovered successfully. Unlock to continue.");
    } catch (e: unknown) {
      setMsg(`Recovery failed: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  return (
    <div className="min-h-screen flex items-center justify-center p-8" style={{ backgroundColor: "var(--background)", color: "var(--foreground)" }}>
      <main className="max-w-sm w-full flex flex-col gap-4">
        <h1 className="text-3xl font-bold text-center">Gradience Wallet</h1>
        <p className="text-center" style={{ color: "var(--muted-foreground)" }}>Passkey-backed identity</p>

        {step === "login" && (
          <>
            <input
              className="border rounded px-4 py-2"
              style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
              placeholder="Username"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
            />
            <input
              className="border rounded px-4 py-2"
              style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
              type="email"
              placeholder="Email (optional)"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
            />
            <input
              className="border rounded px-4 py-2"
              style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
              type="password"
              placeholder="Vault passphrase (≥12 chars)"
              value={passphrase}
              onChange={(e) => setPassphrase(e.target.value)}
            />
            <div className="flex gap-4">
              <button
                onClick={handleRegister}
                className="flex-1 rounded py-2"
                style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}
              >
                Register
              </button>
              <button
                onClick={handleLogin}
                className="flex-1 border rounded py-2"
                style={{ borderColor: "var(--border)", backgroundColor: "var(--muted)" }}
              >
                Login
              </button>
            </div>
            <p className="text-center text-sm mt-2">
              <button onClick={() => { setStep("forgot"); setMsg(""); }} className="underline" style={{ color: "var(--primary)" }}>
                Forgot Passkey?
              </button>
            </p>

            <div className="mt-2 text-center text-xs" style={{ color: "var(--muted-foreground)" }}>
              Or continue with{" "}
              <a href="/api/auth/oauth/google/start" className="underline" style={{ color: "var(--primary)" }}>Google</a>
              {" / "}
              <a href="/api/auth/oauth/github/start" className="underline" style={{ color: "var(--primary)" }}>GitHub</a>
            </div>
          </>
        )}

        {step === "forgot" && (
          <>
            <input
              className="border rounded px-4 py-2"
              style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
              placeholder="Username"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
            />
            <button
              onClick={handleSendRecovery}
              className="rounded py-2"
              style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}
            >
              Send recovery code
            </button>
            <input
              className="border rounded px-4 py-2"
              style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
              placeholder="Recovery code"
              value={recoveryCode}
              onChange={(e) => setRecoveryCode(e.target.value)}
            />
            <button
              onClick={handleVerifyRecovery}
              className="border rounded py-2"
              style={{ borderColor: "var(--border)", backgroundColor: "var(--muted)" }}
            >
              Verify code
            </button>
            <p className="text-center text-sm">
              <button onClick={() => { setStep("login"); setMsg(""); }} className="underline" style={{ color: "var(--primary)" }}>
                Back to login
              </button>
            </p>
          </>
        )}

        {step === "unlock" && (
          <>
            <p className="text-center text-sm" style={{ color: "var(--muted-foreground)" }}>
              Passkey verified. Unlock your vault to continue.
            </p>
            <input
              className="border rounded px-4 py-2"
              style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
              type="password"
              placeholder="Vault passphrase"
              value={passphrase}
              onChange={(e) => setPassphrase(e.target.value)}
            />
            <button
              onClick={handleUnlock}
              className="rounded py-2"
              style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}
            >
              Unlock Vault
            </button>
            <p className="text-center text-sm">
              <button onClick={() => { setStep("login"); setMsg(""); }} className="underline" style={{ color: "var(--primary)" }}>
                Switch account
              </button>
            </p>
          </>
        )}

        {msg && <p className="text-center text-sm" style={{ color: "#B45309" }}>{msg}</p>}
      </main>
    </div>
  );
}

