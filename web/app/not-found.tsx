"use client";

import Link from "next/link";

export default function NotFound() {
  const hasToken = typeof window !== "undefined" && !!localStorage.getItem("gradience_token");

  return (
    <div
      className="min-h-screen flex flex-col items-center justify-center p-8"
      style={{ backgroundColor: "var(--background)", color: "var(--foreground)" }}
    >
      <h1 className="text-5xl font-bold mb-4">404</h1>
      <p className="text-lg mb-2">Page not found</p>
      <p className="text-sm mb-8" style={{ color: "var(--muted-foreground)" }}>
        The page you are looking for does not exist.
      </p>
      <Link
        href={hasToken ? "/dashboard" : "/login"}
        className="px-6 py-2 rounded"
        style={{ backgroundColor: "var(--primary)", color: "var(--primary-foreground)" }}
      >
        {hasToken ? "Back to Dashboard" : "Go to Login"}
      </Link>
    </div>
  );
}
