import { expect, test } from "@playwright/test";
import { defaultConfig } from "../src/types";

for (const offline of [true, false]) {
  test(`update checks respect the ${offline ? "offline" : "online"} profile and require an install click`, async ({ page }) => {
    const checks: boolean[] = [];
    let installs = 0;
    await page.exposeFunction("updateTestInvoke", (cmd: string, args: { automatic?: boolean }) => {
      if (cmd === "bootstrap") return {
        settings: { locale: "de", updateChecks: "onlineOnly", retentionDays: null, defaultConfig: { ...defaultConfig(), offline } },
        activeMeeting: null, meetings: [], models: [], runtimes: {}, keys: {}, dataDirectory: "test",
      };
      if (cmd === "check_for_update") { checks.push(!!args.automatic); return "1.2.3"; }
      if (cmd === "install_update") { installs++; throw new Error("UPDATE_RECORDING"); }
      return 0;
    });
    await page.addInitScript(() => {
      Object.assign(window, {
        isTauri: true,
        __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener() {} },
        __TAURI_INTERNALS__: {
          transformCallback: () => 1,
          invoke: async (cmd: string, args = {}) => {
            try { return await (window as any).updateTestInvoke(cmd, args); }
            catch (error) { throw (error as Error).message; }
          },
        },
      });
    });
    await page.goto("/");
    await expect(page.getByRole("button", { name: "Neues Meeting starten" })).toBeVisible();
    if (offline) {
      // Observe beyond the two-second startup check, even when no IPC is sent.
      await expect(page.getByText("Hier beginnt dein nächstes gutes Meeting.")).toBeVisible();
      await new Promise(resolve => setTimeout(resolve, 2500));
      expect(checks).toEqual([]);
      await page.getByRole("button", { name: "Einstellungen", exact: true }).click();
      await page.getByRole("button", { name: "Jetzt nach Updates suchen" }).click();
    } else {
      await expect(page.getByText("MeetingPal 1.2.3 ist verfügbar.")).toBeVisible();
      await expect(page.getByRole("button", { name: "Neues Meeting starten" })).toBeVisible();
    }
    const install = page.getByRole("button", { name: "Version 1.2.3 installieren und neu starten" });
    await expect(install).toBeVisible();
    expect(checks).toEqual([!offline]);
    expect(installs).toBe(0);
    await install.click();
    await expect(page.getByRole("alert")).toHaveText("Beende zuerst die Aufnahme.");
    expect(installs).toBe(1);
  });
}
