import { describe, it, expect } from "vitest";
import { nextPhase, updatePct, type UpdatePhase } from "../lib/update";
import type { UpdateInfo } from "../lib/types";

const info = (p: Partial<UpdateInfo>): UpdateInfo => ({ version: "0.3.0", notes: "", ready: false, downloading: false, ...p });

describe("nextPhase", () => {
  it("found maps backend state", () => {
    expect(nextPhase("idle", { type: "found", info: info({ downloading: true }) })).toBe("downloading");
    expect(nextPhase("idle", { type: "found", info: info({ ready: true }) })).toBe("ready");
    expect(nextPhase("idle", { type: "found", info: info({}) })).toBe("failed");
  });
  it("background events advance download phases", () => {
    expect(nextPhase("idle", { type: "progress" })).toBe("downloading");
    expect(nextPhase("downloading", { type: "ready" })).toBe("ready");
    expect(nextPhase("downloading", { type: "downloadFailed" })).toBe("failed");
    expect(nextPhase("failed", { type: "retry" })).toBe("downloading");
  });
  it("scheduled and installing are not overridden by background events", () => {
    for (const p of ["scheduled", "installing"] as UpdatePhase[]) {
      expect(nextPhase(p, { type: "progress" })).toBe(p);
      expect(nextPhase(p, { type: "ready" })).toBe(p);
      expect(nextPhase(p, { type: "downloadFailed" })).toBe(p);
      expect(nextPhase(p, { type: "found", info: info({ ready: true }) })).toBe(p);
    }
  });
  it("user actions", () => {
    expect(nextPhase("ready", { type: "install" })).toBe("installing");
    expect(nextPhase("ready", { type: "scheduled" })).toBe("scheduled");
  });
  it("a failed install returns to ready only when the file is already downloaded", () => {
    expect(nextPhase("installing", { type: "installFailed", ready: true })).toBe("ready");
    expect(nextPhase("installing", { type: "installFailed", ready: false })).toBe("failed");
    expect(nextPhase("idle", { type: "installFailed", ready: false })).toBe("idle");
  });
});

describe("updatePct", () => {
  it("is null without a known total", () => {
    expect(updatePct(null)).toBeNull();
    expect(updatePct({ received: 5, total: null })).toBeNull();
    expect(updatePct({ received: 48, total: 100 })).toBe(48);
  });
});
