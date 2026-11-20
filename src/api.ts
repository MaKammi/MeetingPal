import { invoke, isTauri } from "@tauri-apps/api/core";
import type { Bootstrap } from "./types";
import { defaultConfig } from "./types";
import catalog from "../resources/models.json";
export const desktop = isTauri();
export async function api<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (desktop) return invoke<T>(command, args);
  if (command === "bootstrap")
    return {
      settings: {
        locale: "de",
        defaultConfig: defaultConfig(),
        retentionDays: null,
        updateChecks: "onlineOnly",
      },
      meetings: [],
      models: catalog.map((m) => ({ ...m, installed: false })),
      runtimes: {},
      keys: {},
      dataDirectory: "Desktop app",
      activeMeeting: null,
    } as Bootstrap as T;
  if (command === "list_meetings" || command === "list_video_sources")
    return [] as T;
  if (command === "capture_sources")
    return {
      devices: [],
      sources: [],
      deviceError: null,
      sourceError: null,
    } as T;
  throw new Error(
    "Diese Funktion ist in der installierten Desktop-App verfügbar. / Open the desktop app to use this feature.",
  );
}
