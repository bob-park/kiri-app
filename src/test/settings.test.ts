import { describe, it, expect } from "vitest";
import { defaultSettings, mergeSettings } from "../lib/settings";

describe("mergeSettings", () => {
  it("merges nested objects and replaces arrays", () => {
    const base = { ...defaultSettings, download: { ...defaultSettings.download, subtitles: ["ko"] } };
    const next = mergeSettings(base, { general: { theme: "dark" }, download: { subtitles: [] } });
    expect(next.general.theme).toBe("dark");
    expect(next.general.ui_language).toBe("system");
    expect(next.download.subtitles).toEqual([]);
    expect(next.download.max_concurrent).toBe(2);
  });
});
