import { describe, it, expect } from "vitest";
import { errorText } from "../lib/toast";

const dict: Record<string, string> = {
  "error.invalid_url": "Not YouTube",
  "error.unknown": "Oops",
  "error.no_output": "No file",
};
const t = (k: string, o?: { defaultValue?: string }) => dict[k] ?? o?.defaultValue ?? k;

describe("errorText", () => {
  it("translates command errors by code", () => {
    expect(errorText({ code: "invalid_url", message: "not a YouTube URL" }, t)).toBe("Not YouTube");
  });
  it("falls back to the message for unknown codes", () => {
    expect(errorText({ code: "probe_failed", message: "ERROR: private video" }, t)).toBe("ERROR: private video");
    expect(errorText({ code: "probe_failed", message: "" }, t)).toBe("Oops");
  });
  it("shows the message for code failed and for unknown with a message", () => {
    expect(errorText({ code: "failed", message: "disk full" }, t)).toBe("disk full");
    expect(errorText({ code: "unknown", message: "disk full" }, t)).toBe("disk full");
    expect(errorText({ code: "unknown", message: "" }, t)).toBe("Oops");
  });
  it("translates i18n-key strings and passes raw strings through", () => {
    expect(errorText("error.no_output", t)).toBe("No file");
    expect(errorText(new Error("boom"), t)).toBe("boom");
  });
});
