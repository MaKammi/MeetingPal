import { expect, test, type Page } from "@playwright/test";
import { defaultConfig, type MeetingConfig, type Source } from "../src/types";

const monitor: Source = {
  kind: "screen",
  id: "12",
  name: "Monitor A",
  width: 2560,
  height: 1440,
  isPrimary: true,
};
// Native monitor and window IDs occupy separate namespaces.
const notes: Source = {
  kind: "window",
  id: "12",
  name: "Meeting notes",
  width: 680,
  height: 920,
};

async function setup(page: Page, initial = [monitor, notes]) {
  const backend = {
    sources: initial,
    video: null as MeetingConfig["video"] | null,
    released: [] as string[],
  };
  let grants = 0;
  const frame = (s: Source) => ({
    width: s.width,
    height: s.height,
    image:
      "data:image/svg+xml," +
      encodeURIComponent(
        '<svg xmlns="http://www.w3.org/2000/svg" width="' +
          s.width +
          '" height="' +
          s.height +
          '"><rect width="100%" height="100%" fill="#abc"/></svg>',
      ),
  });
  await page.exposeFunction(
    "sourceTestInvoke",
    (cmd: string, args: Record<string, any>) => {
      switch (cmd) {
        case "bootstrap":
          return {
            settings: {
              locale: "de",
              defaultConfig: defaultConfig(),
              retentionDays: null,
              updateChecks: "manual",
            },
            activeMeeting: null,
            meetings: [],
            models: [],
            runtimes: {},
            keys: {},
            dataDirectory: "test",
          };
        case "capture_sources":
          return {
            sources: backend.sources,
            devices: [],
            deviceError: null,
            sourceError: null,
          };
        case "list_video_sources":
          return backend.sources;
        case "capture_source_preview": {
          const source = backend.sources.find(
            (s) => s.kind === args.kind && s.id === args.id,
          );
          if (!source || source.name === "Unavailable")
            throw new Error("Unavailable");
          return frame(source);
        }
        case "select_portal_source":
          return {
            source: { ...monitor, id: "portal:" + ++grants },
            preview: frame(monitor),
          };
        case "release_portal_source":
          backend.released.push(args.id);
          return;
        case "start_recording":
          backend.video = args.config.video;
          throw new Error("Test received the recording configuration");
        default:
          return 0;
      }
    },
  );
  await page.addInitScript(() => {
    Object.assign(window, {
      isTauri: true,
      __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener() {} },
      __TAURI_INTERNALS__: {
        transformCallback: () => 1,
        invoke: (cmd: string, args = {}) =>
          (window as any).sourceTestInvoke(cmd, args),
      },
    });
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Neues Meeting starten" }).click();
  await page
    .getByPlaceholder("z. B. Produktplanung · November")
    .fill("Auswahltest");
  await page.getByLabel("Bildschirm oder Fenster zusätzlich aufnehmen").check();
  await page.getByRole("button", { name: "Quelle auswählen" }).click();
  return backend;
}

for (const [orientation, screen] of [
  ["landscape", monitor],
  ["portrait", { ...monitor, width: 1440, height: 2560 }],
] as const) {
  test(
    "mouse region on a scaled " +
      orientation +
      " screen reaches the recorder in physical pixels",
    async ({ page }) => {
      const backend = await setup(page, [screen, notes]);
      const screens = page.getByRole("region", { name: "Bildschirme" });
      const windows = page.getByRole("region", { name: "Fenster" });
      await expect(
        screens.getByRole("img", { name: "Vorschau von Bildschirm 1" }),
      ).toBeVisible();
      await expect(
        windows.getByRole("img", { name: "Vorschau von Meeting notes" }),
      ).toBeVisible();
      await windows.locator("label").click();
      await expect(
        page.getByRole("button", { name: "Bereich aufziehen" }),
      ).toHaveCount(0);
      await page.getByRole("button", { name: "Vorschau", exact: true }).click();
      await expect(
        page.getByRole("heading", { name: "Quellenvorschau" }),
      ).toBeVisible();
      await page.keyboard.press("Escape");
      await screens.locator("label").click();
      await page.getByRole("button", { name: "Bereich aufziehen" }).click();
      const stage = page.getByRole("button", {
        name: "Aufnahmebereich in der Bildschirmvorschau auswählen",
      });
      await expect(stage).toBeVisible();
      const box = (await stage.boundingBox())!;
      const pt = (fx: number, fy: number) => ({
        x: Math.round(box.x + fx * box.width),
        y: Math.round(box.y + fy * box.height),
      });
      const pixels = (p: { x: number; y: number }) => ({
        x: Math.round(((p.x - box.x) * screen.width) / box.width),
        y: Math.round(((p.y - box.y) * screen.height) / box.height),
      });
      const drag = async (
        from: { x: number; y: number },
        to: { x: number; y: number },
      ) => {
        await page.mouse.move(from.x, from.y);
        await page.mouse.down();
        await page.mouse.move(to.x, to.y, { steps: 4 });
        await page.mouse.up();
      };
      const a = pt(0.25, 0.25),
        b = pt(0.65, 0.65),
        c = pt(0.75, 0.75);
      await drag(b, a); // Drag up and left, then enlarge at the bottom-right corner.
      await drag(b, c);
      const start = pt(0.5, 0.5),
        end = pt(0.56, 0.56);
      await drag(start, end);
      const crop = {
        x: pixels(a).x + pixels(end).x - pixels(start).x,
        y: pixels(a).y + pixels(end).y - pixels(start).y,
        width: pixels(c).x - pixels(a).x,
        height: pixels(c).y - pixels(a).y,
      };
      await page.getByRole("button", { name: "Bereich übernehmen" }).click();
      await expect(
        page.getByText(
          "Ausschnitt · " + crop.width + " × " + crop.height + " px",
        ),
      ).toBeVisible();
      // Closing a second draft leaves the applied region intact.
      await page.getByRole("button", { name: "Quelle ändern" }).click();
      await windows.locator("label").click();
      await page.keyboard.press("Escape");
      await page.getByLabel(/Die erforderliche Einwilligung/).check();
      await page
        .getByRole("button", { name: "Aufnahme starten", exact: true })
        .click();
      await expect
        .poll(() => backend.video)
        .toEqual({ kind: "region", id: screen.id, crop });
    },
  );
}

test("refresh drops a vanished window and a window remains usable without a thumbnail", async ({
  page,
}) => {
  const backend = await setup(page);
  await page.getByRole("region", { name: "Fenster" }).locator("label").click();
  backend.sources = [monitor];
  await page.getByRole("button", { name: "Quellen aktualisieren" }).click();
  await expect(page.getByText("Keine Fenster verfügbar.")).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Quelle übernehmen" }),
  ).toBeDisabled();
  backend.sources = [monitor, { ...notes, name: "Unavailable" }];
  await page.getByRole("button", { name: "Quellen aktualisieren" }).click();
  await expect(page.getByText("Vorschau nicht verfügbar")).toBeVisible();
  await page.getByRole("region", { name: "Fenster" }).locator("label").click();
  await page.getByRole("button", { name: "Quelle übernehmen" }).click();
  await page.getByLabel(/Die erforderliche Einwilligung/).check();
  await page
    .getByRole("button", { name: "Aufnahme starten", exact: true })
    .click();
  await expect
    .poll(() => backend.video)
    .toEqual({ kind: "window", id: notes.id, crop: null });
});

test("a Wayland grant stays assigned to its region and a cancelled replacement is released", async ({
  page,
}) => {
  const backend = await setup(page, [
    { ...monitor, id: "portal", kind: "portal" },
  ]);
  const portal = page.getByRole("button", {
    name: "Bildschirm oder Fenster im System auswählen",
  });
  await portal.click();
  await page.getByRole("button", { name: "Bereich aufziehen" }).click();
  const stage = page.getByRole("button", {
    name: "Aufnahmebereich in der Bildschirmvorschau auswählen",
  });
  await stage.focus();
  await page.keyboard.press("Enter");
  await page.getByRole("button", { name: "Bereich übernehmen" }).click();
  await page.getByRole("button", { name: "Quelle ändern" }).click();
  await portal.click();
  await expect(
    page.getByRole("radio", { name: "Freigegebener Bildschirm" }),
  ).toBeChecked();
  await page.keyboard.press("Escape");
  await expect.poll(() => backend.released).toEqual(["portal:2"]);
  await page.getByLabel(/Die erforderliche Einwilligung/).check();
  await page
    .getByRole("button", { name: "Aufnahme starten", exact: true })
    .click();
  await expect
    .poll(() => backend.video)
    .toEqual({
      kind: "portal",
      id: "portal:1",
      crop: { x: 640, y: 360, width: 1280, height: 720 },
    });
});
