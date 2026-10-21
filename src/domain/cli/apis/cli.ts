import type { CliStatus } from '@/domain/cli/apis/cli.dto';
import call from '@/shared/api/call';

export async function getCliStatus() {
  return call<CliStatus>('cli_status');
}

/** Links `/usr/local/bin/kiri` to the bundled CLI (asks for an admin password). */
export async function installCli() {
  return call<void>('install_cli');
}

export async function uninstallCli() {
  return call<void>('uninstall_cli');
}
