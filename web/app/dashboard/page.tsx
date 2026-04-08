"use client";

import { useCallback, useEffect, useState } from "react";
import { apiGet, apiPost, setApiBase } from "@/lib/api";
import { registerPasskey } from "@/lib/webauthn";
import { SecureVault } from "@/lib/secureVault";
import { Capacitor } from "@capacitor/core";
import { generateMnemonic } from "bip39";
import WalletCard from "./components/WalletCard";
import type { Wallet } from "./types";

interface PendingApproval {
  status: string;
}

export default function Dashboard() {
  const [wallets, setWallets] = useState<Wallet[]>([]);
  const [name, setName] = useState("");
  const [loading, setLoading] = useState(false);
  const [msg, setMsg] = useState("");
  const [apiBase, setApiBaseState] = useState("");
  const [showApiConfig, setShowApiConfig] = useState(false);
  const [pendingApprovals, setPendingApprovals] = useState(0);
  const [mounted, setMounted] = useState(false);
  const [needsPassphrase, setNeedsPassphrase] = useState(false);
  const [passphrase, setPassphrase] = useState("");
  const [confirmPassphrase, setConfirmPassphrase] = useState("");
  const [unlocking, setUnlocking] = useState(false);
  const [unlockError, setUnlockError] = useState("");
  const [username, setUsername] = useState("");
  const [showBindPasskey, setShowBindPasskey] = useState(false);
  const [bindPassphrase, setBindPassphrase] = useState("");
  const [bindLoading, setBindLoading] = useState(false);
  const [showFaceID, setShowFaceID] = useState(false);
  const [faceIDPassphrase, setFaceIDPassphrase] = useState("");
  const [faceIDLoading, setFaceIDLoading] = useState(false);
  const [recoveryPhrase, setRecoveryPhrase] = useState("");
  const [showRecovery, setShowRecovery] = useState(false);
  const [recoverySaved, setRecoverySaved] = useState(false);
  const [isNative, setIsNative] = useState(false);

  const tryAutoUnlock = useCallback(async () => {
    try {
      const { key } = await SecureVault.retrieveKey();
      await apiPost("/api/auth/unlock", { passphrase: key });
      setNeedsPassphrase(false);
      await fetchWallets();
    } catch {
      // ignore, leave needsPassphrase true so modal shows
    }
  }, []);

  useEffect(() => {
    setMounted(true);
    setIsNative(Capacitor.isNativePlatform());
  }, []);

  useEffect(() => {
    setMounted(true);
    if (typeof window !== "undefined") {
      const saved = localStorage.getItem("gradience_api_base") || "http://localhost:8080";
      setApiBaseState(saved);
      const isProdApi = saved.trim().startsWith("https://api.gradiences.xyz");
      setShowApiConfig(window.location.protocol === "https:" && saved.startsWith("http:") && !isProdApi);
    }
    apiGet("/api/auth/me")
      .then((res) => res.json())
      .then((data) => {
        if (data.username) setUsername(data.username);
        if (!data.has_passphrase) {
          setNeedsPassphrase(true);
          if (Capacitor.isNativePlatform()) {
            tryAutoUnlock();
          }
        }
      })
      .catch(() => {
        // 401 will be handled by handleAuthError redirect
      });
  }, [tryAutoUnlock]);

  async function fetchWallets() {
    try {
      const res = await apiGet("/api/wallets");
      const data = await res.json();
      setWallets(data);
    } catch (e: unknown) {
      setMsg(`Failed to load wallets: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  async function fetchPendingApprovals() {
    try {
      const res = await apiGet("/api/policy-approvals");
      const data: PendingApproval[] = await res.json();
      setPendingApprovals(data.filter((a) => a.status === "pending").length);
    } catch {
      // ignore
    }
  }

  useEffect(() => {
    fetchWallets();
    fetchPendingApprovals();
  }, []);

  async function handleCreate() {
    if (!name.trim()) return;
    setLoading(true);
    try {
      await apiPost("/api/wallets", { name });
      setName("");
      await fetchWallets();
    } catch (e: unknown) {
      setMsg(`Create failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setLoading(false);
    }
  }

  async function handleEnableFaceID() {
    if (faceIDPassphrase.length < 12) {
      setMsg("Passphrase must be at least 12 characters");
      return;
    }
    setFaceIDLoading(true);
    setMsg("");
    try {
      await apiPost("/api/auth/unlock", { passphrase: faceIDPassphrase });
      const arr = new Uint8Array(32);
      crypto.getRandomValues(arr);
      const masterKey = Array.from(arr).map((b) => b.toString(16).padStart(2, "0")).join("");
      const phrase = generateMnemonic(256);
      await SecureVault.storeKey({ key: masterKey });
      setRecoveryPhrase(phrase);
      setShowFaceID(false);
      setShowRecovery(true);
    } catch (e: unknown) {
      setMsg(`Enable failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setFaceIDLoading(false);
    }
  }

  async function handleUnlock() {
    if (passphrase.length < 12) {
      setUnlockError("Passphrase must be at least 12 characters");
      return;
    }
    if (passphrase !== confirmPassphrase) {
      setUnlockError("Passphrases do not match");
      return;
    }
    setUnlocking(true);
    setUnlockError("");
    try {
      await apiPost("/api/auth/unlock", { passphrase });
      setNeedsPassphrase(false);
      await fetchWallets();
    } catch (e: unknown) {
      setUnlockError(`Unlock failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setUnlocking(false);
    }
  }

  async function handleLogout() {
    try {
      await apiPost("/api/auth/logout", {});
    } catch {
      // ignore
    }
    localStorage.removeItem("gradience_token");
    window.location.href = "/login";
  }

  async function handleBindPasskey() {
    if (!username || bindPassphrase.length < 12) {
      setMsg("Please enter your current passphrase (≥12 chars)");
      return;
    }
    setBindLoading(true);
    setMsg("");
    try {
      await registerPasskey(username, bindPassphrase);
      setShowBindPasskey(false);
      setBindPassphrase("");
      localStorage.setItem("gradience_username", username);
      setMsg("Passkey bound successfully");
    } catch (e: unknown) {
      setMsg(`Bind failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setBindLoading(false);
    }
  }

  return (
    <div className="min-h-screen p-8 max-w-4xl mx-auto" style={{ backgroundColor: "var(--background)", color: "var(--foreground)" }}>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold">Wallet Dashboard</h1>
        <div className="flex items-center gap-3">
          <a
            href="/policies"
            className="text-sm px-3 py-1.5 rounded"
            style={{ backgroundColor: "var(--muted)", color: "var(--foreground)", border: "1px solid var(--border)" }}
          >
            Policies
          </a>
          <a
            href="/approvals"
            className="text-sm px-3 py-1.5 rounded relative"
            style={{ backgroundColor: "var(--muted)", color: "var(--foreground)", border: "1px solid var(--border)" }}
          >
            Approvals
            {pendingApprovals > 0 && (
              <span
                className="absolute -top-1.5 -right-1.5 text-xs font-bold px-1.5 py-0.5 rounded-full"
                style={{ backgroundColor: "#F59E0B", color: "#fff" }}
              >
                {pendingApprovals}
              </span>
            )}
          </a>
          <button
            onClick={() => setShowBindPasskey(true)}
            className="text-sm px-3 py-1.5 rounded"
            style={{ backgroundColor: "var(--muted)", color: "var(--foreground)", border: "1px solid var(--border)" }}
          >
            Passkey
          </button>
          {isNative && (
            <button
              onClick={() => setShowFaceID(true)}
              className="text-sm px-3 py-1.5 rounded"
              style={{ backgroundColor: "var(--muted)", color: "var(--foreground)", border: "1px solid var(--border)" }}
            >
              FaceID
            </button>
          )}
          {username && (
            <span className="text-sm hidden sm:inline" style={{ color: "var(--muted-foreground)" }}>{username}</span>
          )}
          <button
            onClick={handleLogout}
            className="text-sm px-3 py-1.5 rounded font-medium"
            style={{ backgroundColor: "#EF4444", color: "#fff" }}
          >
            Log out
          </button>
        </div>
      </div>

      <div className="flex gap-2 mb-6">
        <input
          className="border rounded px-3 py-2 flex-1"
          style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
          placeholder="Wallet name"
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
        <button
          onClick={handleCreate}
          disabled={loading}
          className="px-4 py-2 rounded disabled:opacity-50"
          style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}
        >
          {loading ? "Creating..." : "Create Wallet"}
        </button>
      </div>

      {mounted && showApiConfig && (
        <div className="mb-4 border rounded p-3" style={{ backgroundColor: "#FEF3C7", borderColor: "#FDE68A" }}>
          <p className="text-sm text-yellow-900 mb-2">
            You are on HTTPS but your local API is HTTP. Please enter your local API tunnel URL (e.g. ngrok HTTPS) or switch to local dev mode.
          </p>
          <div className="flex gap-2">
            <input
              className="border rounded px-2 py-1 flex-1 text-sm"
              style={{ backgroundColor: "var(--card)", borderColor: "var(--border)" }}
              placeholder="https://your-ngrok-url.ngrok-free.app"
              value={apiBase}
              onChange={(e) => setApiBaseState(e.target.value)}
            />
            <button
              onClick={() => { setApiBase(apiBase); setShowApiConfig(false); setMsg("API base updated. Refreshing..."); window.location.reload(); }}
              className="px-3 py-1 rounded text-sm"
              style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}
            >
              Save
            </button>
          </div>
        </div>
      )}

      {msg && <p className="text-sm mb-4" style={{ color: "#B45309" }}>{msg}</p>}

      <div className="space-y-6">
        {wallets.length === 0 && <p style={{ color: "var(--muted-foreground)" }}>No wallets yet.</p>}
        {wallets.map((w) => (
          <WalletCard key={w.id} wallet={w} />
        ))}
      </div>

      {needsPassphrase && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4" style={{ backgroundColor: "rgba(0,0,0,0.5)" }}>
          <div className="w-full max-w-md rounded-lg p-6 shadow-xl" style={{ backgroundColor: "var(--card)", border: "1px solid var(--border)" }}>
            <h2 className="text-xl font-bold mb-2">Set your vault passphrase</h2>
            <p className="text-sm mb-4" style={{ color: "var(--muted-foreground)" }}>
              This passphrase encrypts your local vault. Make sure to remember it — it cannot be recovered.
            </p>
            <input
              className="border rounded px-3 py-2 w-full mb-3"
              style={{ backgroundColor: "var(--background)", borderColor: "var(--border)", color: "var(--foreground)" }}
              type="password"
              placeholder="Passphrase (≥12 chars)"
              value={passphrase}
              onChange={(e) => setPassphrase(e.target.value)}
            />
            <input
              className="border rounded px-3 py-2 w-full mb-3"
              style={{ backgroundColor: "var(--background)", borderColor: "var(--border)", color: "var(--foreground)" }}
              type="password"
              placeholder="Confirm passphrase"
              value={confirmPassphrase}
              onChange={(e) => setConfirmPassphrase(e.target.value)}
            />
            <button
              onClick={handleUnlock}
              disabled={unlocking || passphrase.length < 12 || confirmPassphrase !== passphrase}
              className="w-full rounded py-2 disabled:opacity-50"
              style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}
            >
              {unlocking ? "Setting..." : "Unlock Vault & Continue"}
            </button>
            {unlockError && <p className="text-sm mt-2" style={{ color: "#B45309" }}>{unlockError}</p>}
          </div>
        </div>
      )}
      {showBindPasskey && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4" style={{ backgroundColor: "rgba(0,0,0,0.5)" }}>
          <div className="w-full max-w-sm rounded-lg p-6 shadow-xl" style={{ backgroundColor: "var(--card)", border: "1px solid var(--border)" }}>
            <h2 className="text-xl font-bold mb-2">Bind Passkey</h2>
            <p className="text-sm mb-4" style={{ color: "var(--muted-foreground)" }}>
              Add a Passkey to this account for faster and more secure sign-ins.
            </p>
            <input
              className="border rounded px-3 py-2 w-full mb-3"
              style={{ backgroundColor: "var(--background)", borderColor: "var(--border)", color: "var(--foreground)" }}
              type="password"
              placeholder="Current passphrase (≥12 chars)"
              value={bindPassphrase}
              onChange={(e) => setBindPassphrase(e.target.value)}
            />
            <div className="flex gap-3">
              <button
                onClick={handleBindPasskey}
                disabled={bindLoading || bindPassphrase.length < 12}
                className="flex-1 rounded py-2 disabled:opacity-50"
                style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}
              >
                {bindLoading ? "Binding..." : "Confirm & Bind"}
              </button>
              <button
                onClick={() => { setShowBindPasskey(false); setBindPassphrase(""); }}
                className="flex-1 rounded py-2"
                style={{ backgroundColor: "var(--muted)", color: "var(--foreground)", border: "1px solid var(--border)" }}
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}

      {showFaceID && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4" style={{ backgroundColor: "rgba(0,0,0,0.5)" }}>
          <div className="w-full max-w-sm rounded-lg p-6 shadow-xl" style={{ backgroundColor: "var(--card)", border: "1px solid var(--border)" }}>
            <h2 className="text-xl font-bold mb-2">Enable FaceID / Biometric</h2>
            <p className="text-sm mb-4" style={{ color: "var(--muted-foreground)" }}>
              Use your device biometric to unlock the vault without typing passphrase.
            </p>
            <input
              className="border rounded px-3 py-2 w-full mb-3"
              style={{ backgroundColor: "var(--background)", borderColor: "var(--border)", color: "var(--foreground)" }}
              type="password"
              placeholder="Current passphrase (≥12 chars)"
              value={faceIDPassphrase}
              onChange={(e) => setFaceIDPassphrase(e.target.value)}
            />
            <div className="flex gap-3">
              <button
                onClick={handleEnableFaceID}
                disabled={faceIDLoading || faceIDPassphrase.length < 12}
                className="flex-1 rounded py-2 disabled:opacity-50"
                style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}
              >
                {faceIDLoading ? "Enabling..." : "Enable"}
              </button>
              <button
                onClick={() => { setShowFaceID(false); setFaceIDPassphrase(""); }}
                className="flex-1 rounded py-2"
                style={{ backgroundColor: "var(--muted)", color: "var(--foreground)", border: "1px solid var(--border)" }}
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}

      {showRecovery && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4" style={{ backgroundColor: "rgba(0,0,0,0.5)" }}>
          <div className="w-full max-w-md rounded-lg p-6 shadow-xl" style={{ backgroundColor: "var(--card)", border: "1px solid var(--border)" }}>
            <h2 className="text-xl font-bold mb-2">Save Recovery Phrase</h2>
            <p className="text-sm mb-4" style={{ color: "var(--muted-foreground)" }}>
              This is the only way to recover your wallet if you switch devices or reinstall the app. Write it down and keep it safe.
            </p>
            <div className="rounded p-4 mb-4 font-mono text-sm text-center" style={{ backgroundColor: "var(--muted)", color: "var(--foreground)" }}>
              {recoveryPhrase}
            </div>
            <label className="flex items-center gap-2 mb-4 text-sm" style={{ color: "var(--foreground)" }}>
              <input
                type="checkbox"
                checked={recoverySaved}
                onChange={(e) => setRecoverySaved(e.target.checked)}
              />
              I have written it down
            </label>
            <button
              onClick={() => { setShowRecovery(false); setRecoveryPhrase(""); setRecoverySaved(false); setMsg("FaceID enabled. Recovery phrase saved."); }}
              disabled={!recoverySaved}
              className="w-full rounded py-2 disabled:opacity-50"
              style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}
            >
              Done
            </button>
          </div>
        </div>
      )}

    </div>
  );
}

