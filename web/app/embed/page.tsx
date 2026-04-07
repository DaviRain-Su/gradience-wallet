"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { apiGet, apiPost } from "@/lib/api";
import {
  isTrustedOrigin,
  postToParent,
  type EmbedMessage,
  type EmbedMethod,
} from "@/lib/messenger";

interface Wallet {
  id: string;
  name: string;
}

interface PendingRequest {
  id: string;
  method: EmbedMethod;
  origin: string;
  params?: Record<string, unknown>;
}

export default function EmbedWallet() {
  const [token, setToken] = useState<string | null>(null);
  const [wallets, setWallets] = useState<Wallet[]>([]);
  const [selectedId, setSelectedId] = useState<string>("");
  const [msg, setMsg] = useState("");
  const [pending, setPending] = useState<PendingRequest | null>(null);
  const [resultMsg, setResultMsg] = useState<string>("");
  const handledRef = useRef<Set<string>>(new Set());

  useEffect(() => {
    const t = localStorage.getItem("gradience_token");
    setToken(t);
  }, []);

  useEffect(() => {
    if (!token) return;
    apiGet("/api/wallets")
      .then((r) => r.json())
      .then((data: Wallet[]) => {
        setWallets(data);
        if (data.length > 0) setSelectedId(data[0].id);
      })
      .catch((e) => setMsg(`Load wallets failed: ${e.message}`));
  }, [token]);

  const handleConnect = useCallback(
    (req: PendingRequest) => {
      const wallet = wallets.find((w) => w.id === selectedId);
      postToParent({
        id: req.id,
        result: {
          connected: true,
          walletId: wallet?.id || null,
          walletName: wallet?.name || null,
        },
      });
      setResultMsg(`Connected to ${req.origin}`);
    },
    [selectedId, wallets]
  );

  const handleGetBalance = useCallback(
    async (req: PendingRequest) => {
      const walletId =
        typeof req.params?.walletId === "string"
          ? req.params.walletId
          : selectedId;
      if (!walletId) {
        postToParent({
          id: req.id,
          error: { code: -32000, message: "No wallet selected" },
        });
        return;
      }
      try {
        const res = await apiGet(`/api/wallets/${walletId}/portfolio`);
        const data = (await res.json()) as Array<{
          chain_id: string;
          native_balance: string;
          assets: unknown[];
        }>;
        const simplified = data.map((p) => ({
          chain_id: p.chain_id,
          native_balance: p.native_balance,
          assets: p.assets,
        }));
        postToParent({
          id: req.id,
          result: { walletId, portfolio: simplified },
        });
        setResultMsg(`Balance sent for ${walletId}`);
      } catch (e) {
        const err = e instanceof Error ? e.message : String(e);
        postToParent({
          id: req.id,
          error: { code: -32000, message: err },
        });
        setResultMsg(`Balance error: ${err}`);
      }
    },
    [selectedId]
  );

  useEffect(() => {
    function onMessage(event: MessageEvent<unknown>) {
      if (!isTrustedOrigin(event.origin)) return;

      const data = event.data as Partial<EmbedMessage>;
      if (!data.id || !data.method) return;
      if (handledRef.current.has(data.id)) return;
      handledRef.current.add(data.id);

      const req: PendingRequest = {
        id: data.id,
        method: data.method as EmbedMethod,
        origin: event.origin,
        params: data.params,
      };

      if (req.method === "connect" || req.method === "ping") {
        handleConnect(req);
        return;
      }

      if (req.method === "get_balance") {
        void handleGetBalance(req);
        return;
      }

      if (req.method === "sign_transaction") {
        setPending(req);
        return;
      }

      postToParent({
        id: req.id,
        error: { code: -32601, message: `Method not found: ${req.method}` },
      });
    }

    window.addEventListener("message", onMessage);
    return () => window.removeEventListener("message", onMessage);
  }, [handleConnect, handleGetBalance]);

  async function approveSign() {
    if (!pending || pending.method !== "sign_transaction") return;
    const walletId =
      typeof pending.params?.walletId === "string"
        ? pending.params.walletId
        : selectedId;
    const chainId =
      typeof pending.params?.chainId === "string"
        ? pending.params.chainId
        : "eip155:8453";
    const txRaw = pending.params?.transaction;
    const tx =
      txRaw && typeof txRaw === "object" && !Array.isArray(txRaw)
        ? (txRaw as Record<string, string>)
        : undefined;

    if (!walletId || !tx) {
      postToParent({
        id: pending.id,
        error: { code: -32000, message: "Missing parameters" },
      });
      setPending(null);
      return;
    }
    try {
      const res = await apiPost(`/api/wallets/${walletId}/sign`, {
        chain_id: chainId,
        transaction: tx,
      });
      const data = (await res.json()) as unknown;
      postToParent({ id: pending.id, result: data });
      setResultMsg(`Signed tx for ${walletId}`);
    } catch (e) {
      const err = e instanceof Error ? e.message : String(e);
      postToParent({
        id: pending.id,
        error: { code: -32000, message: err },
      });
      setResultMsg(`Sign error: ${err}`);
    }
    setPending(null);
  }

  function rejectSign() {
    if (!pending) return;
    postToParent({
      id: pending.id,
      error: { code: -32000, message: "User rejected" },
    });
    setResultMsg("Signature rejected by user");
    setPending(null);
  }

  if (!token) {
    return (
      <div className="min-h-screen flex items-center justify-center p-6 bg-gray-950 text-gray-100">
        <div className="max-w-sm w-full text-center space-y-4">
          <h1 className="text-xl font-bold text-indigo-400">Gradience Embedded Wallet</h1>
          <p className="text-sm text-gray-400">
            Please log in to Gradience Wallet first to use the embedded mode.
          </p>
          <a
            href="/"
            className="inline-block bg-indigo-600 hover:bg-indigo-500 text-white px-4 py-2 rounded text-sm"
          >
            Open Gradience Wallet
          </a>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen p-4 bg-gray-950 text-gray-100">
      <div className="max-w-md mx-auto space-y-4">
        <div className="flex items-center justify-between">
          <h1 className="text-lg font-bold text-indigo-400">Gradience Wallet</h1>
          <span className="text-xs text-gray-500">Embedded</span>
        </div>

        <div className="bg-gray-900 border border-gray-800 rounded-lg p-3">
          <label className="text-xs text-gray-400">Active Wallet</label>
          <select
            className="mt-1 w-full bg-gray-950 border border-gray-700 rounded px-2 py-1 text-sm"
            value={selectedId}
            onChange={(e) => setSelectedId(e.target.value)}
          >
            {wallets.map((w) => (
              <option key={w.id} value={w.id}>
                {w.name}
              </option>
            ))}
          </select>
        </div>

        {resultMsg && (
          <div className="text-xs text-gray-400 bg-gray-900 border border-gray-800 rounded p-2">
            {resultMsg}
          </div>
        )}

        {pending && (
          <div className="bg-gray-900 border border-yellow-700 rounded-lg p-4 space-y-3">
            <p className="text-sm font-medium text-yellow-400">
              Signature Request
            </p>
            <p className="text-xs text-gray-400">From: {pending.origin}</p>
            <div className="text-xs font-mono bg-gray-950 border border-gray-800 rounded p-2">
              {JSON.stringify(pending.params?.transaction, null, 2)}
            </div>
            <div className="flex gap-2">
              <button
                onClick={rejectSign}
                className="flex-1 border border-gray-700 bg-gray-800 hover:bg-gray-700 text-gray-200 rounded py-2 text-sm"
              >
                Reject
              </button>
              <button
                onClick={approveSign}
                className="flex-1 bg-indigo-600 hover:bg-indigo-500 text-white rounded py-2 text-sm"
              >
                Approve
              </button>
            </div>
          </div>
        )}

        {!pending && (
          <div className="text-xs text-gray-500 text-center">
            Waiting for requests from trusted dApps...
          </div>
        )}

        {msg && (
          <p className="text-xs text-rose-400 text-center">{msg}</p>
        )}
      </div>
    </div>
  );
}
