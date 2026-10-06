import i18next from "i18next";
import { initReactI18next } from "react-i18next";
import ko from "../locales/ko.json";
import en from "../locales/en.json";
import ja from "../locales/ja.json";
import type { Lang, UiLang } from "./types";

export function resolveLang(pref: UiLang, navigatorLang: string): Lang {
  const code = pref === "system" ? navigatorLang : pref;
  const primary = code.split(/[-_]/)[0]?.toLowerCase();
  return primary === "ko" || primary === "ja" ? primary : "en";
}

/** 자막 언어 코드 → 사람이 읽는 이름 (Intl, 실패하면 코드 그대로). */
export function langName(code: string, uiLang: string): string {
  try {
    return new Intl.DisplayNames([uiLang], { type: "language" }).of(code) ?? code;
  } catch {
    return code;
  }
}

export async function initI18n(lang: Lang) {
  if (i18next.isInitialized) return i18next.changeLanguage(lang);
  return i18next.use(initReactI18next).init({
    lng: lang,
    fallbackLng: "en",
    resources: { ko: { translation: ko }, en: { translation: en }, ja: { translation: ja } },
    interpolation: { escapeValue: false },
  });
}
