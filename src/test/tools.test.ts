import { describe, it, expect } from "vitest";
import { toolsToast } from "../lib/tools";
import type { ToolsStatus } from "../lib/types";

const st = (p: Partial<ToolsStatus>): ToolsStatus => ({ ready: false, installing: false, ytdlp_version: null, error: null, last_check: null, ...p });

describe("toolsToast", () => {
  it("hides once tools are ready", () => {
    expect(toolsToast(st({ ready: true }))).toBe("hidden");
    expect(toolsToast(st({ ready: true, error: "old" }))).toBe("hidden");
  });
  it("shows preparing before status arrives and while installing", () => {
    expect(toolsToast(null)).toBe("preparing");
    expect(toolsToast(st({ installing: true }))).toBe("preparing");
    expect(toolsToast(st({ installing: true, error: "retrying" }))).toBe("preparing");
  });
  it("shows failed when an error remains and nothing is installing", () => {
    expect(toolsToast(st({ error: "offline" }))).toBe("failed");
  });
});
