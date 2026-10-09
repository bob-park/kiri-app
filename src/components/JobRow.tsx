import { useTranslation } from "react-i18next";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { api } from "../lib/tauri";
import { showError } from "../lib/toast";
import { jobDetail, pct } from "../lib/format";
import type { Job } from "../lib/types";
import { Icon } from "./Icon";

export function JobRow({ job }: { job: Job }) {
  const { t } = useTranslation();
  const kind = job.state.kind;
  const running = kind === "downloading" || kind === "encoding";
  const act = (f: () => Promise<unknown>) => () => {
    f().catch(showError);
  };
  const chip = kind === "completed" ? "chip-ok" : kind === "failed" ? "chip-err" : running ? "chip-run" : "chip-idle";

  const btn = "btn btn-ghost btn-sm btn-square";
  return (
    <div className="group surface-card mx-3.5 mb-2 flex items-center gap-3 px-2.5 py-2.5">
      {job.thumbnail ? (
        <img src={job.thumbnail} alt="" className="h-[42px] w-[72px] shrink-0 rounded-lg object-cover" />
      ) : (
        <div className="flex h-[42px] w-[72px] shrink-0 items-center justify-center rounded-lg bg-base-200 text-fg-muted">
          {job.source.kind === "file" && <Icon name="file" />}
        </div>
      )}
      <div className="min-w-0 flex-1">
        <div className="truncate text-[12.5px] font-semibold">{job.title}</div>
        <div className={`truncate text-[11px] ${kind === "failed" ? "text-error" : "text-fg-muted"}`}>{jobDetail(job, t)}</div>
        {running && (
          <progress
            className={`progress mt-1.5 h-[5px] w-full ${kind === "encoding" ? "progress-info" : "progress-primary"}`}
            value={job.progress * 100}
            max={100}
          />
        )}
      </div>
      <span className={`chip ${chip}`}>{running ? pct(job.progress) : t(`state.${kind}`)}</span>
      <div className="flex opacity-50 transition-opacity group-focus-within:opacity-100 group-hover:opacity-100">
        {(running || kind === "queued") && (
          <button className={btn} aria-label={t("job.stop")} title={t("job.stop")} onClick={act(() => api.stopJob(job.id))}><Icon name="pause" className="h-4 w-4" /></button>
        )}
        {(kind === "stopped" || kind === "failed") && (
          <button className={btn} aria-label={t("job.restart")} title={t("job.restart")} onClick={act(() => api.restartJob(job.id))}><Icon name="restart" className="h-4 w-4" /></button>
        )}
        {job.state.kind === "failed" && job.state.message === "error.download_dir_unwritable" && (
          <button className={btn} aria-label={t("app.settings")} title={t("app.settings")} onClick={act(() => api.openSettings())}><Icon name="settings" className="h-4 w-4" /></button>
        )}
        {kind === "completed" && job.output && (
          <button className={btn} aria-label={t("job.reveal")} title={t("job.reveal")} onClick={act(() => revealItemInDir(job.output!))}><Icon name="reveal" className="h-4 w-4" /></button>
        )}
        <button className={btn} aria-label={t("job.remove")} title={t("job.remove")} onClick={act(() => api.removeJob(job.id))}><Icon name="remove" className="h-4 w-4" /></button>
      </div>
    </div>
  );
}
