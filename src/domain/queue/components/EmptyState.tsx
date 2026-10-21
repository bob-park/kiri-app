import Icon from '@/shared/components/Icon';
import useI18n from '@/shared/hooks/useI18n';

import cx from 'classnames';

const KEYCAP =
  'glass shadow-keycap grid h-8 min-w-8 place-items-center rounded-[10px] px-[9px] font-sans text-[14px] font-bold';

interface EmptyStateProps {
  /** Bumped on every accepted ⌘V; each new value replays the keycap press. */
  pressCount: number;
}

/** The empty queue: a floating download orb and the ⌘V hint. */
export default function EmptyState({ pressCount }: Readonly<EmptyStateProps>) {
  // hooks
  const { t } = useI18n();

  // A new key remounts the keycap, which restarts its press animation.
  const press = pressCount > 0 && 'animate-key-press';

  return (
    <div className="flex h-full flex-col items-center justify-center gap-3.5 pb-8 text-center">
      <span className="glass shadow-glass animate-float text-primary grid h-[92px] w-[92px] place-items-center rounded-[28px]">
        <Icon name="download" className="h-9 w-9" />
      </span>
      <span className="flex gap-1.5">
        <kbd key={`cmd-${pressCount}`} className={cx(KEYCAP, press)}>
          ⌘
        </kbd>
        <kbd key={`v-${pressCount}`} className={cx(KEYCAP, press)}>
          V
        </kbd>
      </span>
      <span className="text-muted text-[11.5px]">{t.app.emptyHint}</span>
    </div>
  );
}
