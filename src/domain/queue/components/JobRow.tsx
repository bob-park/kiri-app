import type { Job } from '@/domain/queue/apis/queue.dto';
import { useJobRemove, useJobRestart, useJobStop } from '@/domain/queue/queries/queue';
import { openSettings } from '@/domain/settings/apis/settings';
import type { AppError } from '@/shared/api/common.dto';
import Icon from '@/shared/components/Icon';
import useI18n from '@/shared/hooks/useI18n';
import { useStore } from '@/shared/store/rootStore';
import { jobDetail, pct } from '@/utils/format';

import { revealItemInDir } from '@tauri-apps/plugin-opener';
import cx from 'classnames';

const ICON_BUTTON = 'btn btn-ghost btn-sm btn-square';

interface JobRowProps {
  job: Job;
}

export default function JobRow({ job }: Readonly<JobRowProps>) {
  // store
  const showError = useStore((s) => s.showError);

  // hooks
  const { t } = useI18n();

  // queries
  const { stopJob } = useJobStop();
  const { restartJob } = useJobRestart();
  const { removeJob } = useJobRemove();

  // handle
  const handleOpenSettings = () => void openSettings().catch((e: AppError) => showError(e));
  const handleReveal = (output: string) =>
    void revealItemInDir(output).catch((e: unknown) => showError({ code: 'UNKNOWN', message: String(e) }));

  const kind = job.state.kind;
  const running = kind === 'downloading' || kind === 'encoding';
  const chip = kind === 'completed' ? 'chip-ok' : kind === 'failed' ? 'chip-err' : running ? 'chip-run' : 'chip-idle';
  const output = job.output;

  return (
    <div className="group surface-card mx-3.5 mb-2 flex items-center gap-3 px-2.5 py-2.5">
      {job.thumbnail ? (
        <img src={job.thumbnail} alt="" className="h-[42px] w-[72px] shrink-0 rounded-lg object-cover" />
      ) : (
        <div className="bg-base-200 text-fg-muted flex h-[42px] w-[72px] shrink-0 items-center justify-center rounded-lg">
          {job.source.kind === 'file' && <Icon name="file" />}
        </div>
      )}
      <div className="min-w-0 flex-1">
        <div className="truncate text-[12.5px] font-semibold">{job.title}</div>
        <div
          className={cx('truncate text-[11px]', {
            'text-error': kind === 'failed',
            'text-fg-muted': kind !== 'failed',
          })}
        >
          {jobDetail(job, t)}
        </div>
        {running && (
          <progress
            className={cx('progress mt-1.5 h-[5px] w-full', {
              'progress-info': kind === 'encoding',
              'progress-primary': kind !== 'encoding',
            })}
            value={job.progress * 100}
            max={100}
          />
        )}
      </div>
      <span className={cx('chip', chip)}>{running ? pct(job.progress) : t.state[kind]}</span>
      <div className="flex opacity-50 transition-opacity group-focus-within:opacity-100 group-hover:opacity-100">
        {(running || kind === 'queued') && (
          <button className={ICON_BUTTON} aria-label={t.job.stop} title={t.job.stop} onClick={() => stopJob(job.id)}>
            <Icon name="pause" className="h-4 w-4" />
          </button>
        )}
        {(kind === 'stopped' || kind === 'failed') && (
          <button
            className={ICON_BUTTON}
            aria-label={t.job.restart}
            title={t.job.restart}
            onClick={() => restartJob(job.id)}
          >
            <Icon name="restart" className="h-4 w-4" />
          </button>
        )}
        {job.state.kind === 'failed' && job.state.message === 'error.download_dir_unwritable' && (
          <button
            className={ICON_BUTTON}
            aria-label={t.app.settings}
            title={t.app.settings}
            onClick={handleOpenSettings}
          >
            <Icon name="settings" className="h-4 w-4" />
          </button>
        )}
        {kind === 'completed' && output && (
          <button
            className={ICON_BUTTON}
            aria-label={t.job.reveal}
            title={t.job.reveal}
            onClick={() => handleReveal(output)}
          >
            <Icon name="reveal" className="h-4 w-4" />
          </button>
        )}
        <button
          className={ICON_BUTTON}
          aria-label={t.job.remove}
          title={t.job.remove}
          onClick={() => removeJob(job.id)}
        >
          <Icon name="remove" className="h-4 w-4" />
        </button>
      </div>
    </div>
  );
}
