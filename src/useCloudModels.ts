import { useCallback, useEffect, useState } from "react";
import { api, desktop } from "./api";
import type { Provider } from "./types";

export interface CloudModel {
  id: string;
  inputs: string[];
}

const inFlight = new Map<Provider, Promise<CloudModel[]>>();
export function cloudKeyChanged(provider: Provider) {
  inFlight.delete(provider);
  window.dispatchEvent(
    new CustomEvent("meetingpal:provider-keys-changed", { detail: provider }),
  );
}
function request(provider: Provider) {
  const existing = inFlight.get(provider);
  if (existing) return existing;
  const result = api<CloudModel[]>("provider_model_catalog", {
    provider,
  }).finally(() => {
    if (inFlight.get(provider) === result) inFlight.delete(provider);
  });
  inFlight.set(provider, result);
  return result;
}

export function useCloudModels(provider: Provider, online: boolean) {
  const [revision, setRevision] = useState(0);
  const [result, setResult] = useState<{
    provider: Provider;
    models: CloudModel[];
    error: string;
    loading: boolean;
  }>();
  const refresh = useCallback(() => setRevision((n) => n + 1), []);
  const enabled = online && desktop && provider !== "local";
  useEffect(() => {
    if (!enabled) {
      setResult(undefined);
      return;
    }
    let active = true;
    const started = Date.now();
    setResult((previous) => ({
      provider,
      models: previous?.provider === provider ? previous.models : [],
      error: "",
      loading: true,
    }));
    request(provider).then(
      (models) => {
        if (active) setResult({ provider, models, error: "", loading: false });
      },
      (error) => {
        if (active)
          setResult((previous) => ({
            provider,
            models: previous?.provider === provider ? previous.models : [],
            error: String(error),
            loading: false,
          }));
      },
    );
    const recheck = () => {
      if (!document.hidden && Date.now() - started >= 5 * 60_000) refresh();
    };
    const timer = window.setInterval(recheck, 15 * 60_000);
    window.addEventListener("focus", recheck);
    window.addEventListener("online", refresh);
    const keyChanged = (event: Event) => {
      if ((event as CustomEvent<Provider>).detail === provider) {
        setResult(undefined);
        refresh();
      }
    };
    window.addEventListener("meetingpal:provider-keys-changed", keyChanged);
    return () => {
      active = false;
      window.clearInterval(timer);
      window.removeEventListener("focus", recheck);
      window.removeEventListener("online", refresh);
      window.removeEventListener(
        "meetingpal:provider-keys-changed",
        keyChanged,
      );
    };
  }, [provider, enabled, revision, refresh]);
  const current = enabled && result?.provider === provider ? result : undefined;
  return {
    models: current?.models ?? [],
    error: current?.error ?? "",
    loading: enabled && (current?.loading ?? true),
    refresh,
  };
}
