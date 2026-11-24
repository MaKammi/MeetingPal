import { test, expect, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { defaultConfig } from "../src/types";

async function audit(page: Page) {
  const { violations } = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
    .analyze();
  return violations.flatMap((v) =>
    v.nodes.map((n) => ({
      rule: v.id,
      target: n.target.join(" "),
      detail: n.any.map((c) => c.message).join(" "),
    })),
  );
}

test("library, setup, model catalog and settings meet automated WCAG checks", async ({
  page,
}) => {
  test.setTimeout(75_000);
  await page.goto("/");
  const open = page.getByRole("button", { name: "Neues Meeting starten" });
  await expect(open).toBeVisible();
  const findings = [...(await audit(page))];
  await open.click();
  await expect(page.getByRole("dialog")).toBeVisible();
  findings.push(...(await audit(page)));
  await page.keyboard.press("Escape");
  await expect(open).toBeFocused();
  for (const name of ["KI-Modelle", "Einstellungen"]) {
    await page
      .getByRole("navigation")
      .getByRole("button", { name, exact: true })
      .click();
    findings.push(...(await audit(page)));
  }
  expect(findings).toEqual([]);
});

test("English model selection tests only the selected model and restores keyboard focus", async ({
  page,
}) => {
  const probes: unknown[] = [];
  await page.exposeFunction(
    "fixtureInvoke",
    (cmd: string, args: Record<string, unknown>) => {
      if (cmd === "bootstrap")
        return {
          settings: {
            locale: "en",
            updateChecks: "manual",
            defaultConfig: defaultConfig(),
          },
          activeMeeting: null,
          meetings: [],
          models: [],
          runtimes: {},
          keys: {},
          dataDirectory: "test",
        };
      if (cmd === "capture_sources")
        return {
          devices: [],
          sources: [],
          deviceError: null,
          sourceError: null,
        };
      if (cmd === "probe_model") {
        probes.push(args);
        return { elapsedMs: 1600, requests: 1 };
      }
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
          (window as any).fixtureInvoke(cmd, args),
      },
    }),
  );
  await page.goto("/");
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
  await page.getByRole("button", { name: "Start a new meeting" }).click();
  await page.getByLabel("Process entirely locally").uncheck();
  await page
    .getByLabel("Transcription provider", { exact: true })
    .selectOption("openai");
  await page
    .getByLabel("Transcription model", { exact: true })
    .fill("gpt-4o-transcribe-diarize");
  await expect(page.getByText(/Speakers per section/)).toBeVisible();
  expect(probes).toEqual([]);
  await page
    .getByRole("button", { name: "Test with sample data at provider" })
    .click();
  await expect(
    page.getByRole("status").filter({ hasText: "Test passed" }),
  ).toBeVisible();
  expect(probes).toEqual([
    {
      selection: { provider: "openai", model: "gpt-4o-transcribe-diarize" },
      task: "audio",
    },
  ]);
  await page.keyboard.press("Escape");
  await page.keyboard.press("Control+k");
  await expect(
    page.getByRole("textbox", { name: "Search meetings" }),
  ).toBeFocused();
});
