import { describe, it, expect } from "vitest";
import ko from "../locales/ko.json";
import en from "../locales/en.json";
import ja from "../locales/ja.json";

function keys(obj: Record<string, unknown>, prefix = ""): string[] {
  return Object.entries(obj).flatMap(([k, v]) =>
    typeof v === "object" && v !== null ? keys(v as Record<string, unknown>, `${prefix}${k}.`) : [`${prefix}${k}`],
  );
}

describe("locale files", () => {
  it("have identical key sets", () => {
    const e = keys(en).sort();
    expect(keys(ko).sort()).toEqual(e);
    expect(keys(ja).sort()).toEqual(e);
  });
  it("have no empty strings", () => {
    for (const f of [ko, en, ja]) expect(JSON.stringify(f)).not.toContain('""');
  });
});
