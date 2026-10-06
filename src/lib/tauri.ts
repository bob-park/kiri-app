import { invoke } from "@tauri-apps/api/core";
import type { DeepPartial, Job, NewJob, Settings, ToolsStatus, VideoInfo } from "./types";

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
  toolsStatus: () => invoke<ToolsStatus>("tools_status"),
  updateTools: () => invoke<void>("update_tools"),
  openSettings: () => invoke<void>("open_settings"),
};
