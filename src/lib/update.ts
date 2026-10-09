import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { api } from "./tauri";
import { showError } from "./toast";
import type { UpdateInfo, UpdateProgress } from "./types";

export type UpdatePhase = "idle" | "downloading" | "ready" | "failed" | "scheduled" | "installing";
export type UpdateEvent =
  | { type: "found"; info: UpdateInfo }
  | { type: "progress" }
  | { type: "ready" }
  | { type: "downloadFailed" }
  | { type: "retry" }
  | { type: "scheduled" }
  | { type: "install" }
  | { type: "installFailed"; ready: boolean };

/** 사용자가 고른 설치 흐름은 백그라운드 이벤트로 덮지 않는다. */
const userDriven = (p: UpdatePhase) => p === "scheduled" || p === "installing";

export function nextPhase(p: UpdatePhase, e: UpdateEvent): UpdatePhase {
  switch (e.type) {
    case "found":
      if (userDriven(p)) return p;
      return e.info.ready ? "ready" : e.info.downloading ? "downloading" : "failed";
    case "progress":
      return userDriven(p) ? p : "downloading";
    case "ready":
      return userDriven(p) ? p : "ready";
    case "downloadFailed":
      return userDriven(p) ? p : "failed";
    case "retry":
      return "downloading";
    case "scheduled":
      return "scheduled";
    case "install":
      return "installing";
    case "installFailed":
      return e.ready ? "ready" : "failed";
  }
}

export const updatePct = (p: UpdateProgress | null) => (p?.total ? Math.round((p.received / p.total) * 100) : null);

interface UpdateStore {
  info: UpdateInfo | null;
  phase: UpdatePhase;
  progress: UpdateProgress | null;
  checking: boolean;
  checked: boolean; // 수동 확인을 끝낸 뒤에만 "최신 버전" 을 말한다
  check: () => Promise<void>;
  install: (afterQueue: boolean) => Promise<void>;
  retry: () => void;
  subscribe: () => () => void;
}

// 설치 중 두 번째 요청. 사용자에게 알릴 일이 아니다.
const isBusy = (e: unknown) => (e instanceof Error ? e.message : String(e)) === "busy";

export const useUpdate = create<UpdateStore>((set, get) => {
  const step = (e: UpdateEvent) => set((s) => ({ phase: nextPhase(s.phase, e) }));
  const found = (info: UpdateInfo) => {
    set({ info, progress: null });
    step({ type: "found", info });
  };
  const installFailed = (e: unknown) => {
    if (isBusy(e)) return;
    step({ type: "installFailed", ready: get().info?.ready ?? false });
    showError(e);
  };
  return {
    info: null,
    phase: "idle",
    progress: null,
    checking: false,
    checked: false,
    check: async () => {
      set({ checking: true });
      try {
        const info = await api.checkUpdate();
        set({ checked: true });
        if (info) found(info);
      } catch (e) {
        showError(e);
      } finally {
        set({ checking: false });
      }
    },
    install: async (afterQueue) => {
      if (!afterQueue) step({ type: "install" });
      try {
        await api.installUpdate(afterQueue);
      } catch (e) {
        installFailed(e);
      }
    },
    retry: () => {
      step({ type: "retry" });
      api.retryUpdateDownload().catch(showError);
    },
    subscribe: () => {
      api.updateStatus().then((info) => info && !get().info && found(info)).catch(() => {});
      const subs = [
        listen<UpdateInfo>("update-available", (e) => found(e.payload)),
        listen<UpdateProgress>("update-progress", (e) => {
          set({ progress: e.payload });
          step({ type: "progress" });
        }),
        listen<UpdateInfo>("update-ready", (e) => {
          set({ info: e.payload, progress: null });
          step({ type: "ready" });
        }),
        listen<string>("update-download-failed", () => step({ type: "downloadFailed" })),
        listen<boolean>("update-scheduled", () => step({ type: "scheduled" })),
        listen<string>("update-error", (e) => installFailed(e.payload)),
      ];
      return () => {
        for (const p of subs) p.then((un) => un()).catch(() => {});
      };
    },
  };
});
