import { test, expect } from "@playwright/test";

test("empty library and local model catalog are usable without external page requests", async ({
  page,
}) => {
  const external: string[] = [];
  page.on("request", (request) => {
    if (!request.url().startsWith("http://127.0.0.1"))
      external.push(request.url());
  });
  await page.goto("/");
  await expect(
    page.getByText("Hier beginnt dein nächstes gutes Meeting."),
  ).toBeVisible();
  await page.screenshot({ path: "test-results/library.png", fullPage: true });
  await page.getByRole("button", { name: "KI-Modelle", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: /Whisper small/i }),
  ).toBeVisible();
  await expect(
    page.getByText("Qwen3 VL 4B Instruct · Q4", { exact: true }),
  ).toBeVisible();
  expect(external).toEqual([]);
});

test("setup explains consent, source audio and local/cloud boundaries", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Neues Meeting starten" }).click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await page
    .getByPlaceholder("z. B. Produktplanung · November")
    .fill("Produktplanung");
  await expect(
    page.getByText("Keine Meeting-Daten an Cloud-Anbieter"),
  ).toBeVisible();
  await expect(
    page.getByText(
      "MeetingPal informiert andere Teilnehmer nicht automatisch.",
    ),
  ).toBeVisible();
  await page.getByLabel("Bildschirm oder Fenster zusätzlich aufnehmen").check();
  await expect(
    page.getByText("Der Desktop-Ton kann auch andere Anwendungen enthalten."),
  ).toBeVisible();
  await page.getByLabel("Vollständig lokal verarbeiten").uncheck();
  await page.getByLabel("Transkription provider").selectOption("openai");
  await page.getByLabel("Transkription model").fill("example-transcribe");
  await expect(page.locator(".disclosure-list")).toContainText("OpenAI");
  await page.getByLabel("Vollständig lokal verarbeiten").check();
  await expect(page.getByLabel("Transkription provider")).toHaveValue("local");
  await expect(page.locator(".disclosure-list")).not.toContainText("OpenAI");
  await page.screenshot({ path: "test-results/setup.png", fullPage: true });
  await expect(
    page.getByRole("button", { name: "Aufnahme starten", exact: true }),
  ).toBeDisabled();
});

test("settings disclose secure key storage and disabled retention", async ({
  page,
}) => {
  await page.goto("/");
  await page
    .getByRole("button", { name: "Einstellungen", exact: true })
    .click();
  await expect(page.locator('input[type="password"]')).toHaveCount(3);
  await expect(
    page.locator("select").filter({ has: page.locator('option[value=""]') }),
  ).toHaveValue("");
  await page.screenshot({ path: "test-results/settings.png", fullPage: true });
});
