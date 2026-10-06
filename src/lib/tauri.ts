import { invoke } from "@tauri-apps/api/core";
import type { DeepPartial, Settings } from "./types";

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  patchSettings: (patch: DeepPartial<Settings>) => invoke<void>("patch_settings", { patch }),
};
