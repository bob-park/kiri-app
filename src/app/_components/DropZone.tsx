import Icon from '@/shared/components/Icon';
import useI18n from '@/shared/hooks/useI18n';

import cx from 'classnames';

interface DropZoneProps {
  dragging: boolean;
  onClick: () => void;
}

/** Paste/drop hint. Clicking pastes from the clipboard, like ⌘V. */
export default function DropZone({ dragging, onClick }: Readonly<DropZoneProps>) {
  // hooks
  const { t } = useI18n();

  return (
    <button
      type="button"
      onClick={onClick}
      className={cx(
        'mx-3.5 mt-1.5 mb-2.5 flex items-center gap-3 rounded-[14px] border-[1.5px] border-dashed px-3 py-2.5 text-left transition-colors',
        {
          'border-primary bg-secondary': dragging,
          'border-primary/40 bg-primary/5 hover:bg-primary/10': !dragging,
        },
      )}
    >
      <span className="bg-secondary text-primary grid h-[30px] w-[30px] shrink-0 place-items-center rounded-[10px]">
        <Icon name="download" className="h-4 w-4" />
      </span>
      <span className="min-w-0 flex-1">
        <span className="block truncate text-[12.5px] font-semibold">{t.app.dropTitle}</span>
        <span className="text-fg-muted mt-0.5 block text-[11px]">
          <kbd className="kbd kbd-xs">⌘</kbd> <kbd className="kbd kbd-xs">V</kbd> {t.app.dropShortcut}
        </span>
      </span>
    </button>
  );
}
