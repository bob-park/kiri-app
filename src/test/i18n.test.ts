import { describe, it, expect } from "vitest";
import { resolveLang } from "../lib/i18n";

describe("resolveLang", () => {
  it("uses the navigator language for system", () => {
    expect(resolveLang("system", "ko-KR")).toBe("ko");
    expect(resolveLang("system", "ja")).toBe("ja");
  });
  it("explicit pref wins", () => {
    expect(resolveLang("ja", "ko-KR")).toBe("ja");
  });
  it("falls back to english", () => {
    expect(resolveLang("system", "de-DE")).toBe("en");
  });
});
