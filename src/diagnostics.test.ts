import { describe, expect, it } from "vitest";
import { diagnostic } from "./diagnostics";
import type { Translate } from "./types";

const de: Translate = (german) => german;
const en: Translate = (_, english) => english;

describe("stored and live diagnostics", () => {
  it("localizes recorded warnings without changing the underlying string", () => {
    const original = "Screen recording permission was not granted.";
    expect(diagnostic(original, de)).toContain(
      "Berechtigung zur Bildschirmaufnahme",
    );
    expect(diagnostic(original, en)).toBe(original);
  });
  it("retains HTTP status, dynamic model IDs and retry information", () => {
    expect(
      diagnostic(
        "Retryable: Provider returned HTTP 429. Check the key, model availability and quota.",
        de,
      ),
    ).toMatch(/HTTP 429.*wiederholt/);
    expect(
      diagnostic("No API key available for gemini. Add it in Settings.", de),
    ).toContain("Für gemini fehlt der API-Schlüssel");
  });
  it("preserves unknown operating system diagnostics", () => {
    expect(diagnostic("Audio Driver ABC: E160", de)).toBe(
      "Audio Driver ABC: E160",
    );
  });
});
