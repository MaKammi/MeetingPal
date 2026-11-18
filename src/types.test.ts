import { describe, it, expect } from "vitest";
import { defaultConfig, disclosure, duration } from "./types";
describe("meeting data disclosure", () => {
  it("shows no upload for a completely local profile", () => {
    expect(disclosure(defaultConfig())).toEqual([]);
  });
  it("discloses cloud audio in a separately selected final transcription pass", () => {
    const config=defaultConfig();
    config.finalTranscription={provider:"openai",model:"test"};
    expect(disclosure(config)).toEqual([{task:"final-transcription",provider:"openai",data:"audio"}]);
  });
  it("distinguishes sending a transcript from sending raw audio", () => {
    const config = defaultConfig();
    config.offline = false;
    config.analysis = { provider: "anthropic", model: "test" };
    expect(disclosure(config)).toEqual([
      { task: "analysis", provider: "anthropic", data: "transcript" },
    ]);
  });
  it("includes video with audio only when native video analysis is enabled", () => {
    const config = defaultConfig();
    config.offline = false;
    config.vision = { provider: "gemini", model: "test" };
    expect(disclosure(config)).toEqual([]);
    config.videoAnalysis = "native";
    expect(disclosure(config)[0].data).toBe("video + audio");
    config.videoAnalysis = "frames";
    expect(disclosure(config)[0].data).toBe("images");
  });
});
describe("long meeting clocks", () => {
  it("does not wrap at one hour", () => {
    expect(duration(7_261_000)).toBe("2:01:01");
    expect(duration(61_000)).toBe("01:01");
  });
});
