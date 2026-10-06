import { useTranslation } from "react-i18next";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { api } from "../lib/tauri";
import { showError } from "../lib/toast";
import { jobDetail } from "../lib/format";
import type { Job } from "../lib/types";

export function JobRow({ job }: { job: Job }) {
  const { t } = useTranslation();
  const kind = job.state.kind;
  const running = kind === "downloading" || kind === "encoding";
  const act = (f: () => Promise<unknown>) => () => {
    f().catch(showError);
  };
  const badge = kind === "completed" ? "badge-success" : kind === "failed" ? "badge-error" : "badge-ghost";

  return (
    <div className="flex items-center gap-3 border-b border-base-300 px-4 py-2.5">
      {job.thumbnail ? (
        <img src={job.thumbnail} alt="" className="h-9 w-16 shrink-0 rounded-md object-cover" />
      ) : (
        <div className="h-9 w-16 shrink-0 rounded-md bg-base-300" />
      )}
      <div className="min-w-0 flex-1">
        <div className="truncate text-sm font-semibold">{job.title}</div>
        <div className={`truncate text-xs ${kind === "failed" ? "text-error" : "text-fg-muted"}`}>{jobDetail(job, t)}</div>
        {running && (
          <progress
            className={`progress mt-1 h-1 w-full ${kind === "encoding" ? "progress-info" : "progress-primary"}`}
            value={job.progress * 100}
            max={100}
          />
        )}
      </div>
      {!running && <span className={`badge badge-sm ${badge}`}>{t(`state.${kind}`)}</span>}
      {(running || kind === "queued") && (
        <button className="btn btn-ghost btn-xs" aria-label={t("job.stop")} title={t("job.stop")} onClick={act(() => api.stopJob(job.id))}>⏸</button>
      )}
      {(kind === "stopped" || kind === "failed") && (
        <button className="btn btn-ghost btn-xs" aria-label={t("job.restart")} title={t("job.restart")} onClick={act(() => api.restartJob(job.id))}>↻</button>
      )}
      {job.state.kind === "failed" && job.state.message === "error.download_dir_unwritable" && (
        <button className="btn btn-ghost btn-xs" aria-label={t("app.settings")} title={t("app.settings")} onClick={act(() => api.openSettings())}>⚙</button>
      )}
      {kind === "completed" && job.output && (
        <button className="btn btn-ghost btn-xs" aria-label={t("job.reveal")} title={t("job.reveal")} onClick={act(() => revealItemInDir(job.output!))}>⌕</button>
      )}
      <button className="btn btn-ghost btn-xs" aria-label={t("job.remove")} title={t("job.remove")} onClick={act(() => api.removeJob(job.id))}>✕</button>
    </div>
  );
}
