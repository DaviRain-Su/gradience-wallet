function getApiBase(): string {
  if (typeof window === "undefined") return "http://localhost:8080";
  const env = process.env.NEXT_PUBLIC_API_URL;
  if (env) return env;
  const saved = localStorage.getItem("gradience_api_base");
  if (saved) return saved;
  return "http://localhost:8080";
}

export function apiGetRawBase(): string {
  return getApiBase();
}

export function setApiBase(url: string) {
  if (typeof window !== "undefined") {
    localStorage.setItem("gradience_api_base", url);
  }
}

function getToken(): string | null {
  if (typeof window === "undefined") return null;
  return localStorage.getItem("gradience_token");
}

export async function apiPost(path: string, body: unknown) {
  const token = getToken();
  const base = getApiBase();
  const res = await fetch(`${base}${path}`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    },
    body: JSON.stringify(body),
  });
  if (!res.ok) {
    const text = await res.text().catch(() => "Unknown error");
    throw new Error(text);
  }
  return res;
}

export async function apiGet(path: string) {
  const token = getToken();
  const base = getApiBase();
  const res = await fetch(`${base}${path}`, {
    method: "GET",
    headers: {
      "Content-Type": "application/json",
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    },
  });
  if (!res.ok) {
    const text = await res.text().catch(() => "Unknown error");
    throw new Error(text);
  }
  return res;
}
