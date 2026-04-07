"use client";

import { Suspense } from "react";
import DeviceAuthForm from "./form";

export default function DeviceAuth() {
  return (
    <Suspense fallback={
      <div className="min-h-screen flex items-center justify-center p-8" style={{ backgroundColor: "var(--background)", color: "var(--foreground)" }}>
        <p className="text-sm" style={{ color: "var(--muted-foreground)" }}>Loading...</p>
      </div>
    }>
      <DeviceAuthForm />
    </Suspense>
  );
}
