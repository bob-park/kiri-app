import type { Theme } from "./types";

export function resolveTheme(pref: Theme, systemDark: boolean): "dark" | "light" {
  if (pref === "system") return systemDark ? "dark" : "light";
  return pref;
}

// matchMedia 는 호출마다 새 객체를 주므로 하나만 유지한다(핸들러가 쌓이지 않게).
let mq: MediaQueryList | null = null;
let current: Theme = "system";

function apply() {
  const dark = resolveTheme(current, mq?.matches ?? false) === "dark";
  document.documentElement.dataset.theme = dark ? "kiri" : "kiri-light";
}

export function applyTheme(pref: Theme) {
  current = pref;
  if (!mq) {
    mq = window.matchMedia("(prefers-color-scheme: dark)");
    mq.onchange = apply;
  }
  apply();
}
