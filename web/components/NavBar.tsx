"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

const links = [
  { href: "/dashboard", label: "Dashboard" },
  { href: "/ai", label: "AI" },
  { href: "/workspaces", label: "Workspaces" },
  { href: "/approvals", label: "Approvals" },
];

export default function NavBar() {
  const pathname = usePathname();
  if (pathname === "/") return null;

  return (
    <nav className="border-b" style={{ borderColor: "var(--border)", backgroundColor: "var(--card)" }}>
      <div className="max-w-4xl mx-auto px-4 py-3 flex items-center justify-between">
        <Link href="/dashboard" className="font-bold text-lg" style={{ color: "var(--foreground)" }}>
          Gradience
        </Link>
        <div className="flex gap-4">
          {links.map((l) => (
            <Link
              key={l.href}
              href={l.href}
              className={`text-sm transition-colors ${pathname === l.href ? "font-semibold" : ""}`}
              style={{ color: pathname === l.href ? "var(--primary)" : "var(--muted-foreground)" }}
            >
              {l.label}
            </Link>
          ))}
        </div>
      </div>
    </nav>
  );
}
