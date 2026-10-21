import type { Settings, SettingsPatch } from '@/domain/settings/apis/settings.dto';
import call from '@/shared/api/call';

export async function getSettings() {
  return call<Settings>('get_settings');
}

/** Rust merges the patch, saves it and emits `settings://changed` to every window. */
export async function patchSettings(patch: SettingsPatch) {
  return call<Settings>('patch_settings', { patch });
}

/** Shows (or focuses) the settings window. */
export async function openSettings() {
  return call<void>('open_settings');
}
