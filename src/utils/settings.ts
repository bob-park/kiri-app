// Type-only import: erased at runtime, so node --test needs no alias resolution.
import type { Settings, SettingsPatch } from '@/domain/settings/apis/settings.dto';

/** Used only when `get_settings` fails at startup. Mirrors Rust `Settings::default()`. */
export const DEFAULT_SETTINGS: Settings = {
  version: 1,
  general: { ui_language: 'system', theme: 'system', close_to_tray: true },
  download: {
    dir: '',
    quality: 'best',
    preset: 'original',
    subtitles: [],
    skip_sheet: false,
    max_concurrent: 2,
    hw_accel: true,
  },
  update: { auto_check: true, last_ytdlp_check: null },
};

function isObj(v: unknown): v is Record<string, unknown> {
  return typeof v === 'object' && v !== null && !Array.isArray(v);
}

function merge<T>(base: T, patch: unknown): T {
  const out: Record<string, unknown> = { ...(base as Record<string, unknown>) };
  for (const [k, v] of Object.entries(patch as Record<string, unknown>)) {
    if (v === undefined) {
      continue;
    }
    out[k] = isObj(v) && isObj(out[k]) ? merge(out[k], v) : v;
  }

  return out as T;
}

/** Objects merge key by key; arrays and scalars replace (same rule as Rust `patch`). */
export function mergeSettings(base: Settings, patch: SettingsPatch): Settings {
  return merge(base, patch);
}

/**
 * `settings://changed` echoes back to the window that patched. Patches not yet on disk
 * are laid over each echo, so an older echo never undoes a newer optimistic patch.
 */
export function createPendingPatches() {
  let inFlight = 0;
  let merged: SettingsPatch | null = null;

  return {
    begin(patch: SettingsPatch) {
      inFlight++;
      merged = merged ? merge(merged, patch) : patch;
    },
    end() {
      inFlight--;
      if (inFlight === 0) {
        merged = null;
      }
    },
    overlay(settings: Settings): Settings {
      return merged ? mergeSettings(settings, merged) : settings;
    },
  };
}
