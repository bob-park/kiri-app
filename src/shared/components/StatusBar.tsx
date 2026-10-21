import { Fragment } from 'react';

import type { Job } from '@/domain/queue/apis/queue.dto';
import { useUpdateDownloadRetry, useUpdateInstall, useUpdateStatus } from '@/domain/updater/queries/updater';
import useI18n from '@/shared/hooks/useI18n';
import { queueSummary } from '@/utils/queue';
import { updatePct } from '@/utils/update';

const POPOVER_ID = 'update-install';

interface UpdateStatusProps {
  /** Jobs still queued or running. */
  pending: number;
}

function UpdateStatus({ pending }: Readonly<UpdateStatusProps>) {
  // hooks
  const { t } = useI18n();

  // queries
  const { info, phase, progress } = useUpdateStatus();
  const { installUpdate } = useUpdateInstall();
  const { retryUpdateDownload } = useUpdateDownloadRetry();

  // handle
  const hidePopover = () => document.getElementById(POPOVER_ID)?.hidePopover();
  const handleInstall = (afterQueue: boolean) => {
    hidePopover();
    installUpdate(afterQueue);
  };

  if (!info || phase === 'idle') {
    return null;
  }

  const version = info.version;
  const ring = <span className="loading loading-spinner loading-xs text-primary" />;

  if (phase === 'downloading') {
    const percent = updatePct(progress);

    return (
      <span className="flex items-center gap-1.5">
        {ring}
        {t.update.downloadingBg(version)}
        {percent !== null && ` ${percent}%`}
      </span>
    );
  }
  if (phase === 'installing') {
    return (
      <span className="flex items-center gap-1.5">
        {ring}
        {t.update.installing}
      </span>
    );
  }
  if (phase === 'scheduled') {
    return <span>{t.update.scheduled}</span>;
  }
  if (phase === 'failed') {
    return (
      <span className="flex items-center gap-2">
        <span className="text-error">{t.update.downloadFailed}</span>
        <button className="btn btn-ghost btn-xs" onClick={() => retryUpdateDownload()}>
          {t.update.retry}
        </button>
      </span>
    );
  }

  // ready
  return (
    <span className="bg-secondary text-secondary-content flex items-center gap-1.5 rounded-lg py-0.5 pr-0.5 pl-2 font-semibold">
      <span className="bg-primary h-[7px] w-[7px] rounded-full" />
      {t.update.ready(version)}
      {pending === 0 ? (
        <button className="btn btn-primary btn-xs" onClick={() => installUpdate(false)}>
          {t.update.restartInstall}
        </button>
      ) : (
        <>
          <button className="btn btn-primary btn-xs" popoverTarget={POPOVER_ID}>
            {t.update.installMenu}
          </button>
          <div
            id={POPOVER_ID}
            popover="auto"
            style={{ inset: 'auto', right: 12, bottom: 40 }}
            className="glass float-shadow text-base-content fixed m-0 w-[260px] rounded-[14px] p-3 font-normal"
          >
            <div className="text-[12.5px] font-bold">{t.update.readyTitle(version)}</div>
            <p className="text-muted mt-1 text-[11px] leading-relaxed">{t.update.busyDesc(pending)}</p>
            <div className="mt-2.5 flex gap-1.5">
              <button className="btn btn-sm flex-1" onClick={() => handleInstall(false)}>
                {t.update.installNow}
              </button>
              <button className="btn btn-primary btn-sm flex-1" onClick={() => handleInstall(true)}>
                {t.update.afterQueue}
              </button>
            </div>
          </div>
        </>
      )}
    </span>
  );
}

interface StatusBarProps {
  jobs: Job[];
}

/** The main window's footer: queue counts on the left, app update state on the right. */
export default function StatusBar({ jobs }: Readonly<StatusBarProps>) {
  // hooks
  const { t } = useI18n();

  const summary = queueSummary(jobs);
  const pending = summary.filter(([k]) => k === 'running' || k === 'queued').reduce((a, [, n]) => a + n, 0);

  return (
    <footer className="glass text-muted relative flex h-8 shrink-0 items-center gap-2.5 border-x-0 border-b-0 px-3.5 text-[11px] tabular-nums">
      {summary.map(([k, n], i) => (
        <Fragment key={k}>
          {i > 0 && <span className="bg-base-content/10 h-3 w-px" />}
          <span>{t.status[k](n)}</span>
        </Fragment>
      ))}
      <span className="flex-1" />
      <span role="status">
        <UpdateStatus pending={pending} />
      </span>
    </footer>
  );
}
