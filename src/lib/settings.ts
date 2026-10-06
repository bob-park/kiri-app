import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { api } from "./tauri";
import { showError } from "./toast";
import type { DeepPartial, Settings } from "./types";

export const defaultSettings: Settings = {
  version: 1,
  general: { ui_language: "system", theme: "system", close_to_tray: true },
  download: {
    dir: "",
    quality: "best",
    preset: "original",
    subtitles: [],
    skip_sheet: false,
    max_concurrent: 2,
    hw_accel: true,
  },
  update: { auto_check: true, last_ytdlp_check: null },
};

function isObj(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

function merge<T>(base: T, patch: DeepPartial<T>): T {
  const out: Record<string, unknown> = { ...(base as Record<string, unknown>) };
  for (const [k, v] of Object.entries(patch as Record<string, unknown>)) {
    if (v === undefined) continue;
    out[k] = isObj(v) && isObj(out[k]) ? merge(out[k], v) : v;
  }
  return out as T;
}

export const mergeSettings = (base: Settings, patch: DeepPartial<Settings>): Settings => merge(base, patch);

interface SettingsStore {
  settings: Settings | null;
  load: () => Promise<void>;
  update: (patch: DeepPartial<Settings>) => Promise<void>;
  subscribeBackend: () => () => void;
}

// settings-changed 는 호출자에게도 되돌아온다. 아직 디스크에 닿지 않은 패치는 에코 위에 다시 덮는다.
let pending = 0;
let pendingPatch: DeepPartial<Settings> | null = null;

export const useSettings = create<SettingsStore>((set, get) => ({
  settings: null,
  load: async () => {
    try {
      set({ settings: await api.getSettings() });
    } catch (e) {
      set({ settings: get().settings ?? defaultSettings });
      showError(e);
    }
  },
  update: async (patch) => {
    const prev = get().settings;
    set({ settings: mergeSettings(prev ?? defaultSettings, patch) });
    pending++;
    pendingPatch = pendingPatch ? merge(pendingPatch, patch) : patch;
    try {
      await api.patchSettings(patch);
    } catch (e) {
      set({ settings: prev });
      showError(e);
    } finally {
      pending--;
      if (pending === 0) pendingPatch = null;
    }
  },
  subscribeBackend: () => {
    const p = listen<Settings>("settings-changed", (e) => {
      set({ settings: pendingPatch ? mergeSettings(e.payload, pendingPatch) : e.payload });
    });
    return () => {
      p.then((un) => un());
    };
  },
}));
