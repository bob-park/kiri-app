import { describe, it, expect } from "vitest";
import { extractUrl } from "../lib/paste";

describe("extractUrl", () => {
  it("finds the first http(s) url", () => {
    expect(extractUrl("https://youtu.be/x")).toBe("https://youtu.be/x");
    expect(extractUrl("  watch this https://www.youtube.com/watch?v=a then")).toBe("https://www.youtube.com/watch?v=a");
    expect(extractUrl("# comment\nhttps://youtu.be/y\n")).toBe("https://youtu.be/y"); // text/uri-list
  });
  it("returns null without a url", () => {
    expect(extractUrl("hello")).toBeNull();
    expect(extractUrl("")).toBeNull();
    expect(extractUrl(null)).toBeNull();
  });
});
