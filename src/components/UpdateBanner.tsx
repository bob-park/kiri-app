import { useTranslation } from "react-i18next";
import { useUpdate } from "../lib/update";
import { isIdle, useQueue } from "../lib/queue";

export function UpdateBanner() {
  const { t } = useTranslation();
  const { info, progress, scheduled, install } = useUpdate();
  const busy = useQueue((s) => !isIdle(s.jobs));
  if (!info) return null;
  const pct = progress?.total ? Math.round((progress.received / progress.total) * 100) : 0;
  const text = progress
    ? t("update.downloading", { pct })
    : scheduled
      ? t("update.scheduled")
      : t("update.available", { version: info.version });
  return (
    <div role="status" className="mx-3 mt-2 flex items-center gap-2 rounded-xl bg-secondary px-3 py-2 text-sm text-secondary-content">
      <span className="flex-1">{text}</span>
      {!progress && !scheduled && busy && (
        <>
          <button className="btn btn-ghost btn-xs" onClick={() => install(false)}>{t("update.installNow")}</button>
          <button className="btn btn-primary btn-xs" onClick={() => install(true)}>{t("update.afterQueue")}</button>
        </>
      )}
      {!progress && !scheduled && !busy && (
        <button className="btn btn-primary btn-xs" onClick={() => install(false)}>{t("update.install")}</button>
      )}
    </div>
  );
}
