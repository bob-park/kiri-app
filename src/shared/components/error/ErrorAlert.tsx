import { useEffect } from 'react';

import useI18n from '@/shared/hooks/useI18n';
import { useStore } from '@/shared/store/rootStore';
import { errorDetail, errorMessage } from '@/utils/errorMessage';
import { TOAST_MS } from '@/utils/toast';

/**
 * The last failed action (see the error slice), drawn as an error toast card.
 * Render it inside the toast stack; it dismisses itself after `TOAST_MS`.
 */
export default function ErrorAlert() {
  // store
  const error = useStore((s) => s.error);
  const dismissError = useStore((s) => s.dismissError);

  // hooks
  const { t } = useI18n();

  // useEffect
  useEffect(() => {
    if (!error) {
      return;
    }

    const timer = setTimeout(dismissError, TOAST_MS);

    return () => clearTimeout(timer);
  }, [error, dismissError]);

  if (!error) {
    return null;
  }

  const detail = errorDetail(error);

  return (
    <div
      role="alert"
      className="animate-toast-in glass float-shadow text-base-content rounded-xl px-3 py-2.5 text-[12px]"
    >
      <div className="flex items-start gap-2">
        <span className="bg-error mt-[5px] h-2 w-2 shrink-0 rounded-full" />
        <div className="min-w-0 flex-1">
          <div className="font-semibold" title={error.message}>
            {errorMessage(error, t.errors)}
          </div>
          {detail && <div className="mt-0.5 text-[11px] leading-snug opacity-70">{detail}</div>}
        </div>
        <button
          type="button"
          className="cursor-pointer px-1 opacity-60 hover:opacity-100"
          aria-label={t.toast.close}
          onClick={dismissError}
        >
          ×
        </button>
      </div>
    </div>
  );
}
