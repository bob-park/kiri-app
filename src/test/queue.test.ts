import { describe, it, expect } from "vitest";
import { hasActive, isIdle } from "../lib/queue";
import type { Job, JobState } from "../lib/types";

const job = (state: JobState): Job => ({
  id: 1, url: "u", title: "t", thumbnail: null, duration_secs: null, quality_label: "720p",
  options: { format_id: null, preset: "original", subtitles: [], auto_subtitles: false },
  state, progress: 0, speed: null, eta: null, output: null, work_dir: null, created_at: 0,
});

describe("queue helpers", () => {
  it("hasActive only counts running jobs", () => {
    expect(hasActive([job({ kind: "queued" })])).toBe(false);
    expect(hasActive([job({ kind: "encoding" })])).toBe(true);
  });
  it("isIdle is false while anything is queued or running", () => {
    expect(isIdle([job({ kind: "completed" }), job({ kind: "failed", message: "x" })])).toBe(true);
    expect(isIdle([job({ kind: "queued" })])).toBe(false);
    expect(isIdle([])).toBe(true);
  });
});
