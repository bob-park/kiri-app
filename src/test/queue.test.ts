import { describe, it, expect } from "vitest";
import { hasActive, isIdle, queueSummary } from "../lib/queue";
import type { Job, JobState } from "../lib/types";

const job = (state: JobState): Job => ({
  id: 1, source: { kind: "youtube", url: "u" }, title: "t", thumbnail: null, duration_secs: null, quality_label: "720p",
  options: { format_id: null, preset: "original", subtitles: [], auto_subtitles: false, max_height: null },
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
  it("queueSummary groups running states and drops zero counts", () => {
    expect(queueSummary([])).toEqual([]);
    expect(
      queueSummary([
        job({ kind: "downloading" }), job({ kind: "encoding" }), job({ kind: "queued" }),
        job({ kind: "completed" }), job({ kind: "stopped" }),
      ]),
    ).toEqual([["running", 2], ["queued", 1], ["completed", 1]]);
    expect(queueSummary([job({ kind: "failed", message: "x" })])).toEqual([["failed", 1]]);
  });
});
