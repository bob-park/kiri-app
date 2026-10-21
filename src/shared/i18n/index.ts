import { type Dict, en } from '@/shared/i18n/en';
import { ko } from '@/shared/i18n/ko';
import type { Locale } from '@/shared/i18n/locale';

export const dicts: Record<Locale, Dict> = { en, ko };

export type { Dict, Locale };
export { resolveLocale } from '@/shared/i18n/locale';
