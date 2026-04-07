export const TRUSTED_ORIGINS = [
  "http://localhost:3000",
  "http://localhost:3001",
  "http://localhost:5173",
  "http://localhost:8080",
];

export type EmbedMethod =
  | "connect"
  | "get_balance"
  | "sign_transaction"
  | "ping";

export interface EmbedMessage {
  id: string;
  method?: EmbedMethod;
  params?: Record<string, unknown>;
  result?: unknown;
  error?: { code: number; message: string };
}

export function isTrustedOrigin(origin: string): boolean {
  try {
    const url = new URL(origin);
    return TRUSTED_ORIGINS.includes(`${url.protocol}//${url.host}`);
  } catch {
    return false;
  }
}

export function postToParent(msg: EmbedMessage) {
  if (typeof window === "undefined") return;
  window.parent.postMessage(msg, "*");
}

export function postToIframe(
  iframe: HTMLIFrameElement,
  msg: EmbedMessage,
  targetOrigin: string
) {
  if (!iframe.contentWindow) return;
  iframe.contentWindow.postMessage(msg, targetOrigin);
}

export function generateId(): string {
  return `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
}
