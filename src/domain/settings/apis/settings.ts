import type { Settings } from '@/domain/settings/apis/settings.dto';
import call from '@/shared/api/call';

export async function getSettings() {
  return call<Settings>('get_settings');
}
