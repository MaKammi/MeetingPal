import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, desktop } from "./api";
import type { Settings } from "./types";

export function useUpdates(settings: Settings | undefined, recording: boolean) {
  const [version, setVersion] = useState<string | null>(null);
  const [status, setStatus] = useState<"idle" | "checking" | "current" | "installing">("idle");
  const [error, setError] = useState("");
  const [percent, setPercent] = useState<number | null>(null);
  const busy = useRef(false);

  const check = useCallback(async (automatic = false) => {
    if (!desktop || busy.current) return;
    busy.current = true;
    setStatus("checking");
    setError("");
    try {
      const next = await api<string | null>("check_for_update", { automatic });
      setVersion(next);
      setStatus(next ? "idle" : "current");
    } catch (cause) {
      setStatus("idle");
      if (!automatic) setError(String(cause));
    } finally {
      busy.current = false;
    }
  }, []);

  const policy = settings?.updateChecks;
  const offline = settings?.defaultConfig.offline;
  useEffect(() => {
    if (!desktop || recording || !policy || policy === "manual" || (policy === "onlineOnly" && offline)) return;
    const first = window.setTimeout(() => void check(true), 2000);
    const daily = window.setInterval(() => void check(true), 24 * 60 * 60 * 1000);
    return () => { window.clearTimeout(first); window.clearInterval(daily); };
  }, [check, policy, offline, recording]);

  async function install() {
    if (!desktop || !version || busy.current) return;
    busy.current = true;
    setError("");
    setStatus("installing");
    setPercent(null);
    let unlisten: (() => void) | undefined;
    try {
      unlisten = await listen<number>("update-progress", ({ payload }) => setPercent(payload));
      await api("install_update");
    } catch (cause) {
      setError(String(cause));
    } finally {
      unlisten?.();
      busy.current = false;
      setStatus("idle");
    }
  }
  return { version, status, percent, error, check, install };
}

export function updateError(error: string, t: (de: string, en: string) => string) {
  switch (error) {
    case "UPDATE_RECORDING": return t("Beende zuerst die Aufnahme.", "Stop the recording first.");
    case "UPDATE_WORKING": return t("Warte, bis die Verarbeitung abgeschlossen ist, und versuche es erneut.", "Wait for current work to finish and try again.");
    case "UPDATE_BUSY": case "UPDATE_INSTALLING": return t("Ein Updatevorgang läuft bereits.", "An update is already in progress.");
    case "UPDATE_TIMEOUT": return t("GitHub hat nicht rechtzeitig geantwortet.", "GitHub did not respond in time.");
    case "UPDATE_SOURCE": return t("Die Downloadadresse gehört nicht zu diesem Release.", "The download address does not belong to this release.");
    case "UPDATE_CHECK_FIRST": return t("Bitte erneut nach Updates suchen.", "Check for updates again.");
    default: return error;
  }
}
