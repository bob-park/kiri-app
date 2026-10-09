import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { errorText, useToasts, TOAST_MS, withLeaving, type Toast } from "../lib/toast";

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

describe("toast store", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    useToasts.setState({ toasts: [] });
  });
  afterEach(() => vi.useRealTimers());

  const push = useToasts.getState().push;

  it("auto-dismisses plain toasts", () => {
    push({ kind: "info", text: "hi" });
    expect(useToasts.getState().toasts).toHaveLength(1);
    vi.advanceTimersByTime(TOAST_MS);
    expect(useToasts.getState().toasts).toHaveLength(0);
  });

  it("keeps sticky toasts", () => {
    push({ kind: "progress", text: "prep", sticky: true });
    vi.advanceTimersByTime(TOAST_MS * 3);
    expect(useToasts.getState().toasts).toHaveLength(1);
  });

  it("replaces a toast with the same key in place", () => {
    const a = push({ key: "tools", kind: "progress", text: "prep", sticky: true });
    push({ kind: "info", text: "other" });
    const b = push({ key: "tools", kind: "error", text: "failed", sticky: true });
    const { toasts } = useToasts.getState();
    expect(b).toBe(a);
    expect(toasts.map((x) => x.text)).toEqual(["failed", "other"]);
  });

  it("an earlier timer does not close a toast that became sticky", () => {
    push({ key: "k", kind: "info", text: "short" });
    push({ key: "k", kind: "error", text: "long", sticky: true });
    vi.advanceTimersByTime(TOAST_MS * 2);
    expect(useToasts.getState().toasts.map((x) => x.text)).toEqual(["long"]);
  });

  it("dismissKey removes by key and ignores unknown keys", () => {
    push({ key: "tools", kind: "progress", text: "prep", sticky: true });
    useToasts.getState().dismissKey("nope");
    expect(useToasts.getState().toasts).toHaveLength(1);
    useToasts.getState().dismissKey("tools");
    expect(useToasts.getState().toasts).toHaveLength(0);
  });
});

describe("withLeaving", () => {
  const t = (id: number, text = `t${id}`): Toast => ({ id, kind: "info", text });

  it("keeps removed toasts in place, marked leaving, and appends new ones", () => {
    const shown = withLeaving([], [t(1), t(2), t(3)]);
    expect(withLeaving(shown, [t(1), t(3), t(4)])).toEqual([t(1), { ...t(2), leaving: true }, t(3), t(4)]);
  });

  it("updates live toasts in place and revives nothing by accident", () => {
    expect(withLeaving([t(1, "old")], [t(1, "new")])).toEqual([t(1, "new")]);
    expect(withLeaving([{ ...t(1), leaving: true }], [])).toEqual([{ ...t(1), leaving: true }]);
  });
});
