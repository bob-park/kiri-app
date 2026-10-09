import { Fragment } from "react";
import { useTranslation } from "react-i18next";
import { queueSummary, useQueue } from "../lib/queue";
import { updatePct, useUpdate } from "../lib/update";

const POPOVER_ID = "update-install";

function UpdateStatus({ pending }: { pending: number }) {
  const { t } = useTranslation();
  const { info, phase, progress, install, retry } = useUpdate();
  if (!info || phase === "idle") return null;
  const version = info.version;
  const ring = <span className="loading loading-spinner loading-xs text-primary" />;

  if (phase === "downloading") {
    const pct = updatePct(progress);
    return (
      <span className="flex items-center gap-1.5">
        {ring}
        {t("update.downloadingBg", { version })}
        {pct !== null && ` ${pct}%`}
      </span>
    );
  }
  if (phase === "installing") return <span className="flex items-center gap-1.5">{ring}{t("update.installing")}</span>;
  if (phase === "scheduled") return <span>{t("update.scheduled")}</span>;
  if (phase === "failed") {
    return (
      <span className="flex items-center gap-2">
        <span className="text-error">{t("update.downloadFailed")}</span>
        <button className="btn btn-ghost btn-xs" onClick={retry}>{t("update.retry")}</button>
      </span>
    );
  }
  // ready
  const hide = () => document.getElementById(POPOVER_ID)?.hidePopover();
  return (
    <span className="flex items-center gap-1.5 rounded-lg bg-secondary py-0.5 pr-0.5 pl-2 font-semibold text-secondary-content">
      <span className="h-[7px] w-[7px] rounded-full bg-primary" />
      {t("update.ready", { version })}
      {pending === 0 ? (
        <button className="btn btn-primary btn-xs" onClick={() => install(false)}>{t("update.restartInstall")}</button>
      ) : (
        <>
          <button className="btn btn-primary btn-xs" popoverTarget={POPOVER_ID}>{t("update.installMenu")}</button>
          <div
            id={POPOVER_ID}
            popover="auto"
            style={{ inset: "auto", right: 12, bottom: 40 }}
            className="float-shadow fixed m-0 w-[260px] rounded-[14px] border border-base-300 bg-base-100 p-3 font-normal text-base-content"
          >
            <div className="text-[12.5px] font-bold">{t("update.readyTitle", { version })}</div>
            <p className="mt-1 text-[11px] leading-relaxed text-fg-muted">{t("update.busyDesc", { n: pending })}</p>
            <div className="mt-2.5 flex gap-1.5">
              <button className="btn btn-sm flex-1" onClick={() => { hide(); install(false); }}>{t("update.installNow")}</button>
              <button className="btn btn-primary btn-sm flex-1" onClick={() => { hide(); install(true); }}>{t("update.afterQueue")}</button>
            </div>
          </div>
        </>
      )}
    </span>
  );
}

export function StatusBar() {
  const { t } = useTranslation();
  const jobs = useQueue((s) => s.jobs);
  const summary = queueSummary(jobs);
  const pending = summary.filter(([k]) => k === "running" || k === "queued").reduce((a, [, n]) => a + n, 0);
  return (
    <footer className="flex h-8 shrink-0 items-center gap-2.5 border-t border-base-300 bg-base-200 px-3.5 text-[11px] text-fg-muted">
      {summary.map(([k, n], i) => (
        <Fragment key={k}>
          {i > 0 && <span className="h-3 w-px bg-base-300" />}
          <span>{t(`status.${k}`, { n })}</span>
        </Fragment>
      ))}
      <span className="flex-1" />
      <UpdateStatus pending={pending} />
    </footer>
  );
}
