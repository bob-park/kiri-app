import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { api } from "./tauri";
import { showError } from "./toast";
import type { Job, JobState } from "./types";

export const hasActive = (jobs: Job[]) => jobs.some((j) => j.state.kind === "downloading" || j.state.kind === "encoding");
export const isIdle = (jobs: Job[]) => !jobs.some((j) => ["queued", "downloading", "encoding"].includes(j.state.kind));

export type SummaryKey = "running" | "queued" | "completed" | "failed";

/** 상태 막대 왼쪽 요약. 0인 항목은 뺀다. 중지된 작업은 세지 않는다. */
export function queueSummary(jobs: Job[]): [SummaryKey, number][] {
  const count = (...kinds: JobState["kind"][]) => jobs.filter((j) => kinds.includes(j.state.kind)).length;
  const all: [SummaryKey, number][] = [
    ["running", count("downloading", "encoding")],
    ["queued", count("queued")],
    ["completed", count("completed")],
    ["failed", count("failed")],
  ];
  return all.filter(([, n]) => n > 0);
}

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
