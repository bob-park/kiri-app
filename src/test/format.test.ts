import { describe, it, expect } from "vitest";
import { failureText, formatBytes, formatDuration, jobDetail } from "../lib/format";
import type { Job, JobState } from "../lib/types";

const t = (k: string, o?: Record<string, unknown>) => (o && "langs" in o ? `Subs ${o.langs}` : k);
const job = (state: JobState, extra: Partial<Job> = {}): Job => ({
  id: 1, source: { kind: "youtube", url: "u" }, title: "t", thumbnail: null, duration_secs: null, quality_label: "1080p60",
  options: { format_id: "299", preset: "mp4-h264", subtitles: ["ko"], auto_subtitles: false, max_height: null },
  state, progress: 0.48, speed: "2.1 MB/s", eta: "00:12", output: null, work_dir: null, created_at: 0, ...extra,
});

describe("format", () => {
  it("formats bytes and durations", () => {
    expect(formatBytes(null)).toBe("");
    expect(formatBytes(500)).toBe("500 B");
    expect(formatBytes(88_000_000)).toBe("83.9 MB");
    expect(formatBytes(3 * 1024 ** 3)).toBe("3.0 GB");
    expect(formatDuration(144)).toBe("2:24");
    expect(formatDuration(3725)).toBe("1:02:05");
    expect(formatDuration(null)).toBe("");
  });
  it("describes a downloading job", () => {
    expect(jobDetail(job({ kind: "downloading" }), t)).toBe("1080p60 · preset.mp4-h264 · Subs ko · 48% · 2.1 MB/s · 00:12");
  });
  it("describes audio, encoding and failure", () => {
    expect(jobDetail(job({ kind: "encoding" }, { quality_label: "audio", options: { format_id: null, preset: "mp3", subtitles: [], auto_subtitles: false, max_height: null } }), t))
      .toBe("quality.audio · preset.mp3 · state.encoding 48%");
    expect(jobDetail(job({ kind: "failed", message: "ERROR: private" }), t)).toBe("1080p60 · preset.mp4-h264 · Subs ko · ERROR: private");
    expect(failureText("error.no_output", t)).toBe("error.no_output");
  });
  it("labels file transcode jobs", () => {
    const f = job({ kind: "encoding" }, { source: { kind: "file", path: "/a/clip.mkv", output_dir: null }, quality_label: "720p" });
    expect(jobDetail(f, t)).toBe("job.transcode · 720p · preset.mp4-h264 · Subs ko · state.encoding 48%");
  });
});
