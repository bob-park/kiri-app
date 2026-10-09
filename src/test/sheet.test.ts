import { describe, it, expect } from "vitest";
import { buildNewJob, defaultSubtitles, pickDefaultQuality, rememberPatch, visiblePresets } from "../lib/sheet";
import type { Quality, VideoInfo } from "../lib/types";

const q = (format_id: string, height: number, label: string): Quality => ({ format_id, height, fps: null, vcodec: "AVC", filesize: null, label });
const qualities = [q("299", 1080, "1080p60"), q("136", 720, "720p"), q("18", 360, "360p")];
const info: VideoInfo = {
  id: "abc", title: "Rust", channel: "Fireship", duration_secs: 144, thumbnail: "th",
  qualities, subtitles: ["en", "ko"], auto_subtitles: ["en", "ja", "ko"],
};

describe("pickDefaultQuality", () => {
  it("mirrors the backend resolution rules", () => {
    expect(pickDefaultQuality(qualities, "best")?.format_id).toBe("299");
    expect(pickDefaultQuality(qualities, "720p")?.format_id).toBe("136");
    expect(pickDefaultQuality(qualities, "480p")?.format_id).toBe("18");
    expect(pickDefaultQuality(qualities, "144p")?.format_id).toBe("18");
    expect(pickDefaultQuality(qualities, "audio")).toBeNull();
    expect(pickDefaultQuality([], "best")).toBeNull();
  });
});

describe("subtitles", () => {
  it("keeps only languages the video has", () => {
    expect(defaultSubtitles(info, ["ko", "ja", "fr"])).toEqual(["ko", "ja"]);
  });
});

describe("buildNewJob", () => {
  it("video job marks auto subs", () => {
    const j = buildNewJob("u", info, qualities[1], "mp4-h264", ["ko", "ja"]);
    expect(j).toEqual({
      source: { kind: "youtube", url: "u" }, title: "Rust", thumbnail: "th", duration_secs: 144, quality_label: "720p",
      options: { format_id: "136", preset: "mp4-h264", subtitles: ["ko", "ja"], auto_subtitles: true, max_height: null },
    });
  });
  it("audio preset or audio quality drops the video format", () => {
    expect(buildNewJob("u", info, qualities[0], "mp3", []).options.format_id).toBeNull();
    expect(buildNewJob("u", info, null, "original", []).quality_label).toBe("audio");
  });
});

describe("rememberPatch", () => {
  it("stores height-based quality and enables skip_sheet", () => {
    expect(rememberPatch(qualities[0], "mp4-hevc", ["ko"])).toEqual({
      download: { quality: "1080p", preset: "mp4-hevc", subtitles: ["ko"], skip_sheet: true },
    });
    expect(rememberPatch(null, "mp3", []).download?.quality).toBe("audio");
  });
});

describe("visiblePresets", () => {
  it("shows up to three common formats, always including the default, in PRESETS order", () => {
    expect(visiblePresets("original")).toEqual(["original", "mp4-h264", "mp3"]);
    expect(visiblePresets("mp4-h264")).toEqual(["original", "mp4-h264", "mp3"]);
    expect(visiblePresets("webm-vp9")).toEqual(["original", "mp4-h264", "webm-vp9"]);
    expect(visiblePresets("m4a")).toEqual(["original", "mp4-h264", "m4a"]);
  });
});
