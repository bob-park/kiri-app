import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { api } from "./tauri";
import { showError } from "./toast";
import type { Job } from "./types";

export const hasActive = (jobs: Job[]) => jobs.some((j) => j.state.kind === "downloading" || j.state.kind === "encoding");
export const isIdle = (jobs: Job[]) => !jobs.some((j) => ["queued", "downloading", "encoding"].includes(j.state.kind));

interface QueueStore {
  jobs: Job[];
  bind: () => () => void;
}

export const useQueue = create<QueueStore>((set) => ({
  jobs: [],
  bind: () => {
    api.listJobs().then((jobs) => set({ jobs })).catch(showError);
    const p = listen<Job[]>("queue-changed", (e) => set({ jobs: e.payload }));
    return () => {
      p.then((un) => un());
    };
  },
}));
