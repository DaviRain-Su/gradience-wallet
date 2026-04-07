"use client";

import { useEffect, useState } from "react";
import { useSearchParams } from "next/navigation";
import { apiPost } from "@/lib/api";

export default function DeviceAuthForm() {
  const search = useSearchParams();
  const [code, setCode] = useState("");
  const [msg, setMsg] = useState("");
  const [authorized, setAuthorized] = useState(false);

  useEffect(() => {
    const q = search.get("code");
    if (q) setCode(q);
  }, [search]);

  async function handleApprove() {
    try {
      const res = await apiPost("/api/auth/device/authorize", { user_code: code });
      if (!res.ok) throw new Error(await res.text());
      setAuthorized(true);
      setMsg("Device authorized successfully. You can close this tab and return to the CLI.");
    } catch (e: unknown) {
      setMsg(`Authorization failed: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  return (
    <div className="min-h-screen flex items-center justify-center p-8" style={{ backgroundColor: "var(--background)", color: "var(--foreground)" }}>
      <main className="max-w-sm w-full flex flex-col gap-4">
        <h1 className="text-2xl font-bold text-center">Authorize CLI Device</h1>
        <p className="text-center text-sm" style={{ color: "var(--muted-foreground)" }}>
          A CLI or agent is requesting access to your Gradience Wallet.
        </p>

        <input
          className="border rounded px-4 py-2"
          style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
          placeholder="User code"
          value={code}
          onChange={(e) => setCode(e.target.value)}
        />

        {!authorized && (
          <button
            onClick={handleApprove}
            className="rounded py-2"
            style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}
          >
            Approve Device
          </button>
        )}

        {msg && <p className="text-center text-sm" style={{ color: authorized ? "#15803d" : "#B45309" }}>{msg}</p>}
      </main>
    </div>
  );
}
