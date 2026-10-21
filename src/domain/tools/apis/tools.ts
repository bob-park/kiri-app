import type { ToolsStatus } from '@/domain/tools/apis/tools.dto';
import call from '@/shared/api/call';

export async function getToolsStatus() {
  return call<ToolsStatus>('tools_status');
}

/** Checks and installs yt-dlp/Deno now. Progress arrives as `tools://changed`. */
export async function updateTools() {
  return call<void>('update_tools');
}
