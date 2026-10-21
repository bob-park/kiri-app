import type { UpdateInfo } from '@/domain/updater/apis/updater.dto';
import call from '@/shared/api/call';

/** The update found earlier (by the periodic or a manual check), if any. */
export async function getUpdateStatus() {
  return call<UpdateInfo | null>('update_status');
}

export async function checkUpdate() {
  return call<UpdateInfo | null>('check_update');
}

/** With `afterQueue`, only schedules the restart until the queue is idle. */
export async function installUpdate(afterQueue: boolean) {
  return call<void>('install_update', { afterQueue });
}

export async function retryUpdateDownload() {
  return call<void>('retry_update_download');
}
