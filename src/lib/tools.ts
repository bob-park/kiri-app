import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { api } from "./tauri";
import type { ToolsStatus } from "./types";

interface ToolsStore {
  status: ToolsStatus | null;
  bind: () => () => void;
}

export const useTools = create<ToolsStore>((set) => ({
  status: null,
  bind: () => {
    // 이벤트가 invoke 보다 먼저 올 수 있다. 비어 있을 때만 채운다.
    api.toolsStatus().then((s) => set((cur) => ({ status: cur.status ?? s }))).catch(() => {});
    const p = listen<ToolsStatus>("tools-changed", (e) => set({ status: e.payload }));
    return () => {
      p.then((un) => un());
    };
  },
}));
