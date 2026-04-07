"use client";

import { useEffect } from "react";

export default function TgLayout({ children }: { children: React.ReactNode }) {
  useEffect(() => {
    if (document.getElementById("telegram-web-app-script")) return;
    const script = document.createElement("script");
    script.id = "telegram-web-app-script";
    script.src = "https://telegram.org/js/telegram-web-app.js";
    script.async = true;
    script.onload = () => {
      window.Telegram?.WebApp.ready();
      window.Telegram?.WebApp.expand();
    };
    document.body.appendChild(script);
  }, []);

  return (
    <div className="min-h-screen bg-[var(--tg-bg-color,#ffffff)] text-[var(--tg-text-color,#000000)]">
      {children}
    </div>
  );
}
