"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { apiPost } from "@/lib/api";

export default function LoginPage() {
  const [email, setEmail] = useState("");
  const [code, setCode] = useState("");
  const [sent, setSent] = useState(false);
  const [countdown, setCountdown] = useState(0);
  const [msg, setMsg] = useState("");
  const [loading, setLoading] = useState(false);
  const router = useRouter();

  useEffect(() => {
    if (typeof window !== "undefined" && localStorage.getItem("gradience_token")) {
      router.push("/dashboard");
    }
  }, [router]);

  async function handleSendCode() {
    const trimmed = email.trim().toLowerCase();
    if (!trimmed || !trimmed.includes("@")) {
      setMsg("请输入有效的邮箱地址");
      return;
    }
    setLoading(true);
    setMsg("");
    try {
      await apiPost("/api/auth/email/send-code", { email: trimmed });
      setSent(true);
      setCountdown(60);
      const timer = setInterval(() => {
        setCountdown((c) => {
          if (c <= 1) {
            clearInterval(timer);
            return 0;
          }
          return c - 1;
        });
      }, 1000);
    } catch (e: unknown) {
      const text = e instanceof Error ? e.message : String(e);
      if (text.includes("429") || text.includes("TOO_MANY_REQUESTS")) {
        setMsg("发送过于频繁，请 1 分钟后再试，或 1 小时内最多 5 次");
      } else {
        setMsg(`发送失败: ${text}`);
      }
    } finally {
      setLoading(false);
    }
  }

  async function handleVerify() {
    const trimmed = email.trim().toLowerCase();
    if (!trimmed || code.length < 4) {
      setMsg("请输入邮箱和验证码");
      return;
    }
    setLoading(true);
    setMsg("");
    try {
      const res = await apiPost("/api/auth/email/verify", { email: trimmed, code: code.trim() });
      const data = await res.json();
      if (data.token) {
        localStorage.setItem("gradience_token", data.token);
        router.push("/dashboard");
      } else {
        setMsg("登录失败，请重试");
      }
    } catch (e: unknown) {
      const text = e instanceof Error ? e.message : String(e);
      if (text.includes("410") || text.includes("GONE")) {
        setMsg("验证码已过期，请重新发送");
      } else if (text.includes("403") || text.includes("FORBIDDEN")) {
        setMsg("尝试次数过多，请重新发送验证码");
      } else if (text.includes("400") || text.includes("BAD_REQUEST")) {
        setMsg("验证码错误，请检查后重试");
      } else {
        setMsg(`验证失败: ${text}`);
      }
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="min-h-screen flex items-center justify-center p-8" style={{ backgroundColor: "var(--background)", color: "var(--foreground)" }}>
      <main className="max-w-sm w-full flex flex-col gap-4">
        <h1 className="text-3xl font-bold text-center">Gradience Wallet</h1>
        <p className="text-center" style={{ color: "var(--muted-foreground)" }}>Email-backed identity</p>

        <input
          className="border rounded px-4 py-2"
          style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
          type="email"
          placeholder="Enter your email"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
        />

        {!sent ? (
          <button
            onClick={handleSendCode}
            disabled={loading}
            className="rounded py-2 disabled:opacity-50"
            style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}
          >
            {loading ? "Sending..." : "Send verification code"}
          </button>
        ) : (
          <>
            <input
              className="border rounded px-4 py-2"
              style={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
              placeholder="Enter 6-digit code"
              value={code}
              onChange={(e) => setCode(e.target.value)}
            />
            <button
              onClick={handleVerify}
              disabled={loading || code.length < 4}
              className="rounded py-2 disabled:opacity-50"
              style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}
            >
              {loading ? "Verifying..." : "Login / Create account"}
            </button>
            <div className="flex items-center justify-center gap-2">
              <button
                onClick={handleSendCode}
                disabled={loading || countdown > 0}
                className="text-sm underline disabled:no-underline"
                style={{ color: countdown > 0 ? "var(--muted-foreground)" : "var(--primary)" }}
              >
                {countdown > 0 ? `Resend in ${countdown}s` : "Resend code"}
              </button>
            </div>
          </>
        )}

        {msg && <p className="text-center text-sm" style={{ color: "#B45309" }}>{msg}</p>}
      </main>
    </div>
  );
}
