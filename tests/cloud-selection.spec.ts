import { test, expect, type Page } from "@playwright/test";
import { defaultConfig, type MeetingConfig } from "../src/types";

async function fixture(
  page: Page,
  config: MeetingConfig,
  list: (provider: string) => Promise<unknown> | unknown,
) {
  await page.exposeFunction(
    "cloudFixture",
    async (cmd: string, args: Record<string, unknown>) => {
      if (cmd === "bootstrap")
        return {
          settings: {
            locale: "de",
            updateChecks: "manual",
            defaultConfig: config,
          },
          meetings: [],
          models: [],
          runtimes: {},
          keys: { gemini: true, openai: true },
          activeMeeting: null,
          dataDirectory: "test",
        };
      if (cmd === "capture_sources")
        return {
          sources: [],
          devices: [],
          sourceError: null,
          deviceError: null,
        };
      if (cmd === "provider_model_catalog") return list(String(args.provider));
      if (cmd === "probe_model" || cmd === "start_recording")
        throw new Error("Unexpected content transfer");
      return 0;
    },
  );
  await page.addInitScript(() =>
    Object.assign(window, {
      isTauri: true,
      __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener() {} },
      __TAURI_INTERNALS__: {
        transformCallback: () => 1,
        invoke: (cmd: string, args = {}) =>
          (window as any).cloudFixture(cmd, args),
      },
    }),
  );
  await page.goto("/");
  await page.getByRole("button", { name: "Neues Meeting starten" }).click();
}

test("Gemini video can be selected offline, with a cancellable switch and live model discovery", async ({
  page,
}) => {
  const calls: string[] = [];
  let newest = "gemini-4-example";
  await fixture(page, defaultConfig(), (provider) => {
    calls.push(provider);
    return [
      { id: "tts-example", inputs: ["audio"] },
      { id: newest, inputs: ["text", "image", "video"] },
      { id: "gemini-2.5-flash", inputs: ["text", "image", "video"] },
    ];
  });
  await page.getByLabel("Bildschirm oder Fenster zusätzlich aufnehmen").check();
  const mode = page.getByLabel("Visuelle Auswertung");
  const disclosure = page.locator(".disclosure-list");
  await expect(mode.locator('option[value="native"]')).toBeEnabled();
  await mode.selectOption("native");
  const confirm = page.getByRole("dialog", {
    name: "Gemini für dieses Meeting verwenden?",
  });
  await expect(confirm).toContainText(
    "Videoclips einschließlich Ton an Google Gemini",
  );
  await confirm.getByRole("button", { name: "Lokal bleiben" }).click();
  await expect(disclosure).toContainText("Keine Meeting-Daten an Cloud-Anbieter");
  expect(calls).toEqual([]);
  await mode.selectOption("native");
  await confirm.getByRole("button", { name: "Gemini verwenden" }).click();
  await expect(disclosure).toContainText("Google Gemini");
  await expect(mode).toHaveValue("native");
  const model = page.getByLabel("Bild-/Videomodell Modell", { exact: true });
  await expect(model).toHaveValue(newest);
  expect(calls).toEqual(["gemini"]);
  await expect(model.locator("option").first()).toHaveAttribute(
    "value",
    newest,
  );
  await expect(model.locator('option[value="tts-example"]')).toHaveCount(0);
  await expect(page.locator(".disclosure-list")).toContainText("Google Gemini");
  await model.selectOption("gemini-2.5-flash");
  newest = "gemini-5-new";
  await page
    .getByRole("button", { name: "Modelle vom Anbieter laden" })
    .click();
  await expect(model.locator("option").first()).toHaveAttribute(
    "value",
    newest,
  );
  await expect(model).toHaveValue("gemini-2.5-flash");
  await model.selectOption("__custom");
  await model.fill("private-custom-id");
  await expect(model).toHaveValue("private-custom-id");
  await mode.selectOption("frames");
  await page.getByLabel("Bild-/Videomodell Anbieter").selectOption("local");
  await expect(disclosure).toContainText("Keine Meeting-Daten an Cloud-Anbieter");
  await expect(page.getByLabel("Bild-/Videomodell Anbieter")).toHaveValue(
    "local",
  );
});

test("analysis loads and refreshes Gemini from a local meeting, including empty catalogs", async ({ page }) => {
  let reads = 0;
  await fixture(page, defaultConfig(), () => ++reads === 1 ? [] : [{ id: "gemini-current", inputs: ["text"] }]);
  await expect(page.getByLabel("Vollständig lokal verarbeiten")).toHaveCount(0);
  expect(reads).toBe(0);
  await page.getByLabel("Auswertung & Assistenz Anbieter").selectOption("gemini");
  await expect(page.getByText(/Der Anbieter meldet keine passenden Modelle/)).toBeVisible();
  await page.getByRole("button", { name: "Modelle vom Anbieter laden" }).click();
  await expect(page.getByLabel("Auswertung & Assistenz Modell")).toHaveValue("gemini-current");
  await expect(page.locator(".disclosure-list")).toContainText("Transkript");
  await page.getByLabel("Auswertung & Assistenz Anbieter").selectOption("local");
  await expect(page.locator(".disclosure-list")).toContainText("Keine Meeting-Daten an Cloud-Anbieter");
  expect(reads).toBe(2);
});

test("Gemini transcribes both passes and the probe link has its own row", async ({ page }) => {
  const config = defaultConfig();
  config.analysis = { provider: "gemini", model: "legacy-offline-choice" };
  await fixture(page, config, () => [
    { id: "gemini-current", inputs: ["audio", "text"] },
    { id: "gemini-3.5-transcribe", inputs: ["audio"] },
    { id: "gemini-text", inputs: ["text"] },
  ]);
  await expect(page.getByLabel("Auswertung & Assistenz Anbieter")).toHaveValue("local");
  await page.getByLabel("Transkription Anbieter", { exact: true }).selectOption("gemini");
  const model = page.getByLabel("Transkription Modell", { exact: true });
  await expect(model).toHaveValue("gemini-current");
  await expect(model.locator('option[value="gemini-text"]')).toHaveCount(0);
  await model.selectOption("gemini-3.5-transcribe");
  const picker = page.locator(".model-picker").filter({ has: model });
  const features = picker.locator("small").filter({ hasText: "Zeitmarken" });
  const probe = picker.getByRole("button", { name: "Mit Testdaten beim Anbieter prüfen" });
  for (const width of [1920, 900]) {
    await page.setViewportSize({ width, height: 900 });
    await probe.scrollIntoViewIfNeeded();
    const bounds = await features.boundingBox();
    const action = await probe.boundingBox();
    expect(action!.y).toBeGreaterThanOrEqual(bounds!.y + bounds!.height + 4);
    expect(action!.x + action!.width).toBeLessThanOrEqual(width);
  }
  await page.screenshot({ path: "test-results/gemini-transcription.png" });
  await page.getByLabel("Anderes Modell für das endgültige Transkript").check();
  await page.getByLabel("Endgültiges Transkript Anbieter").selectOption("gemini");
  await expect(page.getByLabel("Endgültiges Transkript Modell")).toHaveValue("gemini-current");
  await expect(page.locator(".disclosure-list")).toContainText("Audiomix");
});

test("an older provider response cannot replace the current selection", async ({
  page,
}) => {
  let finish!: (v: unknown) => void;
  const pending = new Promise((r) => {
    finish = r;
  });
  let started = false;
  await fixture(page, { ...defaultConfig(), offline: false }, (provider) => {
    if (provider === "openai") {
      started = true;
      return pending;
    }
    return [{ id: "gemini-new", inputs: ["text"] }];
  });
  const provider = page.getByLabel("Auswertung & Assistenz Anbieter");
  const model = page.getByLabel("Auswertung & Assistenz Modell");
  await provider.selectOption("openai");
  await expect.poll(() => started).toBe(true);
  await provider.selectOption("gemini");
  await expect(model).toHaveValue("gemini-new");
  finish([{ id: "openai-older", inputs: ["text"] }]);
  await expect(model.locator("option").first()).toHaveAttribute(
    "value",
    "gemini-new",
  );
  await expect(model.locator('option[value="openai-older"]')).toHaveCount(0);
});
