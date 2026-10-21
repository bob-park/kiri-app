import type { Job } from '@/domain/queue/apis/queue.dto';
import { useJobRemove, useJobRestart, useJobStop } from '@/domain/queue/queries/queue';
import { openSettings } from '@/domain/settings/apis/settings';
import type { AppError } from '@/shared/api/common.dto';
import Icon from '@/shared/components/Icon';
import useExitAnimation from '@/shared/hooks/useExitAnimation';
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
  const { leaving, exit, cancel, onAnimationEnd } = useExitAnimation('collapse-out');

  // queries
  const { stopJob } = useJobStop();
  const { restartJob } = useJobRestart();
  const { removeJob } = useJobRemove();

  // handle
  const handleOpenSettings = () => void openSettings().catch((e: AppError) => showError(e));
  const handleReveal = (output: string) =>
    void revealItemInDir(output).catch((e: unknown) => showError({ code: 'UNKNOWN', message: String(e) }));
  // The card fades out first; the list drops it when Rust reports the change.
  const handleRemove = () => exit(() => removeJob(job.id, { onError: cancel }));

  const kind = job.state.kind;
  const running = kind === 'downloading' || kind === 'encoding';
  const chip = kind === 'failed' ? 'chip-err' : running ? 'chip-run' : 'chip-idle';
  const output = job.output;

  return (
    <div
      className={cx('group glass shadow-glass mx-3.5 mb-2 flex items-center gap-2.5 rounded-2xl p-2.5', {
        'animate-collapse-out pointer-events-none': leaving,
        'animate-spring-in': !leaving,
      })}
      onAnimationEnd={onAnimationEnd}
    >
      {job.thumbnail ? (
        <img src={job.thumbnail} alt="" className="h-[40px] w-[70px] shrink-0 rounded-[10px] object-cover" />
      ) : (
        <div className="bg-track text-muted flex h-[40px] w-[70px] shrink-0 items-center justify-center rounded-[10px]">
          {job.source.kind === 'file' && <Icon name="file" />}
        </div>
      )}
      <div className="min-w-0 flex-1">
        <div className="truncate text-[12.5px] font-semibold">{job.title}</div>
        <div
          className={cx('mt-0.5 truncate text-[10.5px] tabular-nums', {
            'text-error': kind === 'failed',
            'text-muted': kind !== 'failed',
          })}
        >
          {jobDetail(job, t)}
        </div>
        {(running || kind === 'completed') && (
          <div
            role="progressbar"
            aria-label={job.title}
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={kind === 'completed' ? 100 : Math.round(job.progress * 100)}
            className="bg-track mt-1.5 h-[5px] overflow-hidden rounded-full"
          >
            <div
              className={cx('h-full rounded-full transition-[width] duration-400 motion-reduce:transition-none', {
                'bg-success': kind === 'completed',
                'animate-flow bg-[length:200%_100%]': running,
                'bg-(image:--gradient-progress)': kind === 'downloading',
                'bg-(image:--gradient-progress-encoding)': kind === 'encoding',
              })}
              style={{ width: `${kind === 'completed' ? 100 : job.progress * 100}%` }}
            />
          </div>
        )}
      </div>
      {kind === 'completed' ? (
        // Two elements, because pop (transform) and ring (box-shadow) each need their own `animation`.
        <span className="animate-ring grid shrink-0 rounded-full">
          <span
            role="img"
            aria-label={t.state.completed}
            className="animate-pop bg-success text-success-content grid h-[22px] w-[22px] place-items-center rounded-full text-[12px] font-bold"
          >
            ✓
          </span>
        </span>
      ) : (
        <span className={cx('chip', chip)}>{running ? pct(job.progress) : t.state[kind]}</span>
      )}
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
        <button className={ICON_BUTTON} aria-label={t.job.remove} title={t.job.remove} onClick={handleRemove}>
          <Icon name="remove" className="h-4 w-4" />
        </button>
      </div>
    </div>
  );
}
