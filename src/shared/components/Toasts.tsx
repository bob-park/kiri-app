import { useEffect, useRef, useState } from 'react';

import ErrorAlert from '@/shared/components/error/ErrorAlert';
import useI18n from '@/shared/hooks/useI18n';
import { useStore } from '@/shared/store/rootStore';
import type { ShownToast } from '@/shared/store/toast/toast.state';
import { withLeaving } from '@/utils/toast';

import cx from 'classnames';

const EXIT_MS = 150;

interface ToastCardProps {
  toast: ShownToast;
  onDismiss: (id: number) => void;
}

function ToastCard({ toast, onDismiss }: Readonly<ToastCardProps>) {
  // hooks
  const { t } = useI18n();

  // Only error/info toasts without an action close on click; progress toasts stay until their state changes.
  const clickToClose = !toast.action && toast.kind !== 'progress' && !toast.leaving;

  return (
    <div
      role={toast.leaving ? undefined : toast.kind === 'progress' ? 'status' : 'alert'}
      aria-hidden={toast.leaving || undefined}
      className={cx('glass float-shadow text-base-content rounded-xl px-3 py-2.5 text-[12px]', {
        'animate-toast-out pointer-events-none': toast.leaving,
        'animate-toast-in': !toast.leaving,
        'cursor-pointer': clickToClose,
      })}
      onClick={clickToClose ? () => onDismiss(toast.id) : undefined}
    >
      <div className="flex items-start gap-2">
        {toast.kind === 'progress' ? (
          <span className="loading loading-spinner loading-xs text-primary mt-0.5 shrink-0" />
        ) : (
          <span
            className={cx('mt-[5px] h-2 w-2 shrink-0 rounded-full', {
              'bg-error': toast.kind === 'error',
              'bg-primary': toast.kind !== 'error',
            })}
          />
        )}
        <div className="min-w-0 flex-1">
          <div className="font-semibold">{toast.text}</div>
          {toast.desc && <div className="mt-0.5 text-[11px] leading-snug opacity-70">{toast.desc}</div>}
        </div>
        {toast.action && (
          <>
            <button className="btn btn-primary btn-xs" onClick={toast.action.run}>
              {toast.action.label}
            </button>
            <button
              className="px-1 opacity-60 hover:opacity-100"
              aria-label={t.toast.close}
              onClick={() => onDismiss(toast.id)}
            >
              ×
            </button>
          </>
        )}
      </div>
      {toast.kind === 'progress' && (
        <progress
          className="progress progress-primary bg-track mt-2 h-1 w-full"
          aria-label={toast.text}
          value={toast.progress == null ? undefined : toast.progress * 100}
          max={100}
        />
      )}
    </div>
  );
}

interface ToastsProps {
  /** Distance from the window bottom (px). The main window lifts it above the status bar. */
  bottom?: number;
}

/** The toast stack, with the last failed action (`ErrorAlert`) at its end. */
export default function Toasts({ bottom = 16 }: Readonly<ToastsProps>) {
  // ref
  const stackRef = useRef<HTMLDivElement>(null);
  const liveCount = useRef(0);

  // state
  const [shown, setShown] = useState<ShownToast[]>([]);

  // store
  const toasts = useStore((s) => s.toasts);
  const dismissToast = useStore((s) => s.dismissToast);
  const hasError = useStore((s) => !!s.error);

  // useEffect
  // A dismissed toast stays (as `leaving`) until its exit animation ends.
  useEffect(() => {
    setShown((cur) => withLeaving(cur, toasts));
    const timer = setTimeout(() => setShown((cur) => cur.filter((s) => !s.leaving)), EXIT_MS);

    return () => clearTimeout(timer);
  }, [toasts]);

  // A popover sits in the top layer, so toasts show above a modal <dialog>.
  const visible = shown.length > 0 || hasError;
  useEffect(() => {
    const el = stackRef.current;
    if (!el) {
      return;
    }
    const open = el.matches(':popover-open');
    if (!visible && open) {
      el.hidePopover();
    } else if (visible && !open) {
      el.showPopover();
    }
  }, [visible]);

  // Re-open only when a toast is added, to lift the stack above a modal opened since.
  // Re-opening on every change would replay the enter animation of the toasts still shown.
  const count = toasts.length + (hasError ? 1 : 0);
  useEffect(() => {
    const el = stackRef.current;
    if (el && count > liveCount.current && el.matches(':popover-open')) {
      el.hidePopover();
      el.showPopover();
    }
    liveCount.current = count;
  }, [count]);

  return (
    <div
      ref={stackRef}
      popover="manual"
      style={{ inset: 'auto', right: 16, bottom }}
      className="fixed m-0 flex w-[260px] flex-col gap-2 overflow-visible border-0 bg-transparent p-0"
    >
      {shown.map((x) => (
        <ToastCard key={x.id} toast={x} onDismiss={dismissToast} />
      ))}
      <ErrorAlert />
    </div>
  );
}
