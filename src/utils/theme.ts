// Type-only import: erased at runtime, so node --test needs no alias resolution.
import type { ThemePref } from '@/domain/settings/apis/settings.dto';

export function resolveTheme(pref: ThemePref, prefersDark: boolean): 'light' | 'dark' {
  if (pref === 'system') {
    return prefersDark ? 'dark' : 'light';
  }

  return pref;
}
