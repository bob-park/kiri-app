// Type-only imports: erased at runtime, so node --test needs no alias resolution.
import type { AppError } from '@/shared/api/common.dto';
import type { Dict } from '@/shared/i18n/en';

/** Codes whose `t.errors` text is generic, so the raw message is the useful part. */
const DETAIL_CODES = new Set(['UNKNOWN', 'FAILED', 'PROBE_FAILED']);

/** User-facing text for an `AppError`, keyed by its `code`. Unknown codes get the generic text. */
export function errorMessage(err: AppError, errors: Dict['errors']): string {
  return (errors as Record<string, string>)[err.code] ?? errors.UNKNOWN;
}

/** The raw message to show under `errorMessage` (e.g. yt-dlp's own error), or `undefined`. */
export function errorDetail(err: AppError): string | undefined {
  return DETAIL_CODES.has(err.code) && err.message ? err.message : undefined;
}
