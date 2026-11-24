import { expect, test } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { defaultConfig } from "../src/types";

for (const locale of ["de", "en"] as const) {
  test(`completed meeting and processing states in ${locale}`, async ({
    page,
  }) => {
    test.setTimeout(90_000);
    const isDE = locale === "de";
    const config = { ...defaultConfig(), title: "Project Delta" };
    const source = {
      id: "audio-0",
      startMs: 0,
      endMs: 5000,
      text: "The review is on Tuesday.",
      speaker: "Alex",
      provisional: false,
      edited: false,
    };
    const evidence = { text: source.text, sources: [source.id] };
    const summary = {
      id: "sample",
      title: config.title,
      createdAt: "2026-11-24T10:00:00Z",
      durationMs: 5000,
      tags: ["Review"],
      status: "complete",
      mode: "post",
      offline: true,
    };
    const meeting = {
      ...summary,
      config,
      updatedAt: summary.createdAt,
      notes: "Review",
      markers: [{ atMs: 0, text: "Review" }],
      media: [],
      transcript: [source],
      analysis: {
        summary: [evidence],
        topics: [],
        decisions: [],
        tasks: [],
        questions: [],
        provisional: false,
        generatedAt: summary.createdAt,
      },
      visuals: [],
      answers: [],
      warnings: [],
      cloudRequests: 0,
      tokens: 0,
      videoFiles: [],
      playbackFile: null,
    };
    await page.exposeFunction("detailInvoke", (command: string) => {
      if (command === "bootstrap")
        return {
          settings: { locale, updateChecks: "manual", defaultConfig: config },
          meetings: [summary],
          models: [],
          runtimes: {},
          keys: {},
          dataDirectory: "test",
          activeMeeting: null,
        };
      if (command === "get_meeting") return meeting;
      if (command === "get_jobs")
        return [
          {
            id: "s",
            meetingId: "sample",
            kind: "transcribe",
            payload: source.id,
            state: "done",
            attempts: 1,
            error: null,
          },
        ];
      if (command === "list_meetings") return [summary];
      return 0;
    });
    await page.addInitScript(() =>
      Object.assign(window, {
        isTauri: true,
        __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener() {} },
        __TAURI_INTERNALS__: {
          transformCallback: () => 1,
          invoke: (command: string) => (window as any).detailInvoke(command),
        },
      }),
    );
    await page.goto("/");
    await page.getByRole("button", { name: /Project Delta/ }).click();
    await expect(
      page.getByRole("heading", { name: "Project Delta" }),
    ).toBeVisible();
    const failures: string[] = [];
    for (const section of isDE
      ? [
          "Überblick",
          "Transkript",
          "Eigene Notizen",
          "Meeting fragen",
          "Verarbeitung",
        ]
      : [
          "Overview",
          "Transcript",
          "Your notes",
          "Ask this meeting",
          "Processing",
        ]) {
      await page
        .locator(".tabs")
        .getByRole("button", { name: new RegExp(section) })
        .click();
      const audit = await new AxeBuilder({ page })
        .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
        .analyze();
      for (const v of audit.violations)
        for (const n of v.nodes)
          failures.push(
            [
              section,
              v.id,
              n.target.join(" "),
              ...n.any.map((c) => c.message),
            ].join(" | "),
          );
    }
    await expect(page.locator(".job-row")).toContainText(
      isDE ? "Transkription" : "Transcription",
    );
    await expect(page.locator(".job-row")).toContainText(
      isDE ? "Abgeschlossen" : "Completed",
    );
    await page
      .getByRole("button", {
        name: isDE ? "Meeting löschen" : "Delete meeting",
        exact: true,
      })
      .click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toHaveAccessibleName(
      isDE ? "Meeting endgültig löschen?" : "Permanently delete this meeting?",
    );
    await page.keyboard.press("Escape");
    await expect(dialog).toHaveCount(0);
    expect(failures).toEqual([]);
  });
}
