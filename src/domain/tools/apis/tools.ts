import type { ToolName, ToolsStatus } from '@/domain/tools/apis/tools.dto';
import call from '@/shared/api/call';

export async function getToolsStatus() {
  return call<ToolsStatus>('tools_status');
}

/** Installs every missing tool. Progress arrives as `tools://changed`. */
export async function installTools() {
  return call<void>('install_tools');
}

/** Resolves the latest release of every tool into `ToolInfo.latest`. A failed check is silent. */
export async function checkToolUpdates() {
  return call<void>('check_tool_updates');
}

/** The newer yt-dlp version, if any (hits the network at most hourly). */
export async function checkYtdlpUpdate() {
  return call<string | null>('check_ytdlp_update');
}

/** Installs `names`. Progress arrives as `tools://changed`. */
export async function updateTools(names: ToolName[]) {
  return call<void>('update_tools', { names });
}
