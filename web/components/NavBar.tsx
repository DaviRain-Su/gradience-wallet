"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

const mainLinks = [
  { href: "/dashboard", label: "Dashboard" },
  { href: "/agents", label: "Agents" },
  { href: "/policies", label: "Policies" },
  { href: "/approvals", label: "Approvals" },
  { href: "/activity", label: "Activity" },
];

const rightLink = { href: "/settings", label: "Settings" };

export default function NavBar() {
  const pathname = usePathname();
  if (pathname === "/") return null;

  return (
    <nav className="border-b" style={{ borderColor: "var(--border)", backgroundColor: "var(--card)" }}>
      <div className="max-w-4xl mx-auto px-4 py-3 flex items-center justify-between">
        <Link href="/" className="font-bold text-lg" style={{ color: "var(--foreground)" }}>
          Gradience
        </Link>
        <div className="flex items-center gap-4">
          <div className="flex gap-4">
            {mainLinks.map((l) => (
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
          <Link
            href={rightLink.href}
            className={`text-sm transition-colors ${pathname === rightLink.href ? "font-semibold" : ""}`}
            style={{ color: pathname === rightLink.href ? "var(--primary)" : "var(--muted-foreground)" }}
          >
            {rightLink.label}
          </Link>
        </div>
      </div>
    </nav>
  );
}
