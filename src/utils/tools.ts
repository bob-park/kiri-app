// Type-only import: erased at runtime, so node --test needs no alias resolution.
import type { ToolsStatus } from '@/domain/tools/apis/tools.dto';

export type ToolsToast = 'hidden' | 'preparing' | 'failed';

/** The main window's tools toast: shown only while a tool really downloads, or when tools are missing after a failure. */
export function toolsToast(s: ToolsStatus | null | undefined): ToolsToast {
  if (!s || s.ready) {
    return 'hidden';
  }
  if (s.installing) {
    return 'preparing';
  }

  return s.error ? 'failed' : 'hidden';
}
