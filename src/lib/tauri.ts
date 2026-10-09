import { invoke } from "@tauri-apps/api/core";
import type { CliStatus, DeepPartial, Job, NewJob, Settings, ToolsStatus, UpdateInfo, VideoInfo } from "./types";

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  patchSettings: (patch: DeepPartial<Settings>) => invoke<void>("patch_settings", { patch }),
  listJobs: () => invoke<Job[]>("list_jobs"),
  probe: (url: string) => invoke<VideoInfo>("probe", { url }),
  addJob: (job: NewJob) => invoke<Job>("add_job", { job }),
  addUrl: (url: string) => invoke<Job>("add_url", { url }),
  stopJob: (id: number) => invoke<void>("stop_job", { id }),
  removeJob: (id: number) => invoke<void>("remove_job", { id }),
  restartJob: (id: number) => invoke<void>("restart_job", { id }),
  clearCompleted: () => invoke<number>("clear_completed"),
  toolsStatus: () => invoke<ToolsStatus>("tools_status"),
  updateTools: () => invoke<void>("update_tools"),
  openSettings: () => invoke<void>("open_settings"),
  cliStatus: () => invoke<CliStatus>("cli_status"),
  installCli: () => invoke<void>("install_cli"),
  uninstallCli: () => invoke<void>("uninstall_cli"),
  updateStatus: () => invoke<UpdateInfo | null>("update_status"),
  checkUpdate: () => invoke<UpdateInfo | null>("check_update"),
  installUpdate: (afterQueue: boolean) => invoke<void>("install_update", { afterQueue }),
  retryUpdateDownload: () => invoke<void>("retry_update_download"),
};
