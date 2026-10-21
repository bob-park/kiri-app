// Type-only import: erased at runtime, so node --test needs no alias resolution.
import type { LocalePref } from '@/domain/settings/apis/settings.dto';

export type Locale = 'en' | 'ko';

/**
 * Same rule as Rust `app::i18n::resolve`: `system` follows the OS language.
 * Any value other than `en`/`ko` (e.g. the dropped `ja`) reads as `system`.
 */
export function resolveLocale(pref: LocalePref, systemLanguage: string): Locale {
  if (pref === 'en' || pref === 'ko') {
    return pref;
  }

  return systemLanguage.toLowerCase().startsWith('ko') ? 'ko' : 'en';
}
