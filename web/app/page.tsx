"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { registerPasskey, loginPasskey, unlockVault } from "@/lib/webauthn";

export default function Home() {
  const [username, setUsername] = useState("");
  const [passphrase, setPassphrase] = useState("");
  const [step, setStep] = useState<"login" | "unlock">("login");
  const [msg, setMsg] = useState("");
  const router = useRouter();

  async function handleRegister() {
    try {
      await registerPasskey(username, passphrase);
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

  return (
    <div className="min-h-screen flex items-center justify-center p-8">
      <main className="max-w-sm w-full flex flex-col gap-6">
        <h1 className="text-3xl font-bold text-center">Gradience Wallet</h1>
        <p className="text-center text-gray-500">Passkey-backed identity</p>

        {step === "login" ? (
          <>
            <input
              className="border rounded px-4 py-2"
              placeholder="Username"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
            />
            <input
              className="border rounded px-4 py-2"
              type="password"
              placeholder="Vault passphrase (≥12 chars)"
              value={passphrase}
              onChange={(e) => setPassphrase(e.target.value)}
            />
            <div className="flex gap-4">
              <button
                onClick={handleRegister}
                className="flex-1 bg-black text-white rounded py-2 hover:bg-gray-800"
              >
                Register
              </button>
              <button
                onClick={handleLogin}
                className="flex-1 border rounded py-2 hover:bg-gray-100"
              >
                Login
              </button>
            </div>
          </>
        ) : (
          <>
            <p className="text-center text-sm text-gray-600">
              Passkey verified. Unlock your vault to continue.
            </p>
            <input
              className="border rounded px-4 py-2"
              type="password"
              placeholder="Vault passphrase"
              value={passphrase}
              onChange={(e) => setPassphrase(e.target.value)}
            />
            <button
              onClick={handleUnlock}
              className="bg-black text-white rounded py-2 hover:bg-gray-800"
            >
              Unlock Vault
            </button>
          </>
        )}

        {msg && <p className="text-center text-sm text-red-500">{msg}</p>}
      </main>
    </div>
  );
}
