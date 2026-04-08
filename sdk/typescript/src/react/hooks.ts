import { useCallback, useEffect, useState } from "react";
import { GradienceClient } from "../client";
import type { Wallet, Balance, Policy, SwapQuoteParams, SwapQuoteResult, AiGenerateParams, AiGenerateResult } from "../types";

export interface GradienceReactOptions {
  baseUrl: string;
  apiToken: string;
}

function useClient(opts: GradienceReactOptions) {
  return new GradienceClient(opts.baseUrl, { apiToken: opts.apiToken });
}

export function useWallets(opts: GradienceReactOptions) {
  const [wallets, setWallets] = useState<Wallet[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await useClient(opts).listWallets();
      setWallets(data);
    } catch (e) {
      setError(e as Error);
    } finally {
      setLoading(false);
    }
  }, [opts.baseUrl, opts.apiToken]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { wallets, loading, error, refresh };
}

export function useWalletBalance(opts: GradienceReactOptions, walletId: string | undefined) {
  const [balance, setBalance] = useState<Balance[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const refresh = useCallback(async () => {
    if (!walletId) return;
    setLoading(true);
    setError(null);
    try {
      const data = await useClient(opts).getBalance(walletId);
      setBalance(data);
    } catch (e) {
      setError(e as Error);
    } finally {
      setLoading(false);
    }
  }, [opts.baseUrl, opts.apiToken, walletId]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { balance, loading, error, refresh };
}

export function useCreateWallet(opts: GradienceReactOptions) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const create = useCallback(
    async (name: string) => {
      setLoading(true);
      setError(null);
      try {
        return await useClient(opts).createWallet(name);
      } catch (e) {
        setError(e as Error);
        throw e;
      } finally {
        setLoading(false);
      }
    },
    [opts.baseUrl, opts.apiToken]
  );

  return { create, loading, error };
}

export function usePolicies(opts: GradienceReactOptions, walletId: string | undefined) {
  const [policies, setPolicies] = useState<Policy[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const refresh = useCallback(async () => {
    if (!walletId) return;
    setLoading(true);
    setError(null);
    try {
      const data = await useClient(opts).listPolicies(walletId);
      setPolicies(data);
    } catch (e) {
      setError(e as Error);
    } finally {
      setLoading(false);
    }
  }, [opts.baseUrl, opts.apiToken, walletId]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { policies, loading, error, refresh };
}

export function useCreatePolicy(opts: GradienceReactOptions) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const create = useCallback(
    async (walletId: string, content: string) => {
      setLoading(true);
      setError(null);
      try {
        return await useClient(opts).createPolicy(walletId, content);
      } catch (e) {
        setError(e as Error);
        throw e;
      } finally {
        setLoading(false);
      }
    },
    [opts.baseUrl, opts.apiToken]
  );

  return { create, loading, error };
}

export function useSwapQuote(opts: GradienceReactOptions) {
  const [quote, setQuote] = useState<SwapQuoteResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const fetchQuote = useCallback(
    async (walletId: string, params: SwapQuoteParams) => {
      setLoading(true);
      setError(null);
      try {
        const data = await useClient(opts).swapQuote(walletId, params);
        setQuote(data);
        return data;
      } catch (e) {
        setError(e as Error);
        throw e;
      } finally {
        setLoading(false);
      }
    },
    [opts.baseUrl, opts.apiToken]
  );

  return { quote, loading, error, fetchQuote };
}

export function useAiGenerate(opts: GradienceReactOptions) {
  const [result, setResult] = useState<AiGenerateResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const generate = useCallback(
    async (params: AiGenerateParams) => {
      setLoading(true);
      setError(null);
      try {
        const data = await useClient(opts).aiGenerate(params);
        setResult(data);
        return data;
      } catch (e) {
        setError(e as Error);
        throw e;
      } finally {
        setLoading(false);
      }
    },
    [opts.baseUrl, opts.apiToken]
  );

  return { result, loading, error, generate };
}
