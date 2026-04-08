"use client";

import { useEffect, useState } from "react";
import { usePathname, useRouter } from "next/navigation";

const publicPaths = ["/", "/login", "/embed", "/device", "/tg"];

export default function AuthGuard({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const router = useRouter();
  const [checking, setChecking] = useState(true);

  useEffect(() => {
    if (typeof window !== "undefined") {
      const token = localStorage.getItem("gradience_token");
      if (!token && !publicPaths.includes(pathname)) {
        router.replace("/login");
      } else {
        setChecking(false);
      }
    }
  }, [pathname, router]);

  if (checking && !publicPaths.includes(pathname)) {
    return (
      <div
        className="min-h-screen"
        style={{ backgroundColor: "var(--background)", color: "var(--foreground)" }}
      />
    );
  }

  return <>{children}</>;
}
