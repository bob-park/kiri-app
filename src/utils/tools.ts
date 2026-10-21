// Type-only import: erased at runtime, so node --test needs no alias resolution.
import type { ToolsStatus } from '@/domain/tools/apis/tools.dto';

export type ToolsToast = 'hidden' | 'preparing' | 'failed';

/** The main window's "getting ready to download" toast. */
export function toolsToast(s: ToolsStatus | null | undefined): ToolsToast {
  if (s?.ready) {
    return 'hidden';
  }
  if (s?.error && !s.installing) {
    return 'failed';
  }

  return 'preparing';
}
