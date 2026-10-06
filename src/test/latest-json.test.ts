import { describe, it, expect } from "vitest";
// @ts-expect-error -- 타입 없는 .mjs 스크립트
import { buildManifest } from "../../scripts/latest-json.mjs";

describe("buildManifest", () => {
  it("maps the mac .sig to darwin-aarch64", () => {
    const m = buildManifest("v0.2.0", [{ name: "kiri.app.tar.gz.sig", body: "SIG\n" }], new Date("2026-10-06T00:00:00Z"));
    expect(m).toEqual({
      version: "0.2.0",
      pub_date: "2026-10-06T00:00:00.000Z",
      platforms: {
        "darwin-aarch64": {
          signature: "SIG",
          url: "https://github.com/bob-park/kiri-app/releases/download/v0.2.0/kiri.app.tar.gz",
        },
      },
    });
  });
  it("is empty without signatures", () => {
    expect(buildManifest("v0.2.0", []).platforms).toEqual({});
  });
});
