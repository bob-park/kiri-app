import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { api } from "./tauri";
import { showError } from "./toast";
import type { UpdateInfo, UpdateProgress } from "./types";

interface UpdateStore {
  info: UpdateInfo | null;
  progress: UpdateProgress | null;
  scheduled: boolean;
  checking: boolean;
  checked: boolean; // 수동 확인을 끝낸 뒤에만 "최신 버전" 을 말한다
  check: () => Promise<void>;
  install: (afterQueue: boolean) => Promise<void>;
  subscribe: () => () => void;
}

// 설치 중 두 번째 요청. 사용자에게 알릴 일이 아니다.
const isBusy = (e: unknown) => (e instanceof Error ? e.message : String(e)) === "busy";

export const useUpdate = create<UpdateStore>((set) => ({
  info: null,
  progress: null,
  scheduled: false,
  checking: false,
  checked: false,
  check: async () => {
    set({ checking: true });
    try {
      set({ info: await api.checkUpdate(), checked: true });
    } catch (e) {
      showError(e);
    } finally {
      set({ checking: false });
    }
  },
  install: async (afterQueue) => {
    if (!afterQueue) set((s) => ({ progress: s.progress ?? { received: 0, total: null } }));
    try {
      await api.installUpdate(afterQueue);
    } catch (e) {
      if (isBusy(e)) return;
      set({ progress: null });
      showError(e);
    }
  },
  subscribe: () => {
    api.updateStatus().then((info) => set((s) => ({ info: s.info ?? info }))).catch(() => {});
    const subs = [
      listen<UpdateInfo>("update-available", (e) => set({ info: e.payload })),
      listen<UpdateProgress>("update-progress", (e) => set({ progress: e.payload, scheduled: false })),
      listen<boolean>("update-scheduled", () => set({ scheduled: true })),
      listen<string>("update-error", (e) => {
        if (isBusy(e.payload)) return;
        set({ progress: null });
        showError(e.payload);
      }),
    ];
    return () => {
      for (const p of subs) p.then((un) => un()).catch(() => {});
    };
  },
}));
