import { useId } from 'react';

import cx from 'classnames';

interface SettingRowProps {
  label: string;
  desc?: string;
  /** Put the control under the label at full width (for wide controls such as the theme cards). */
  stacked?: boolean;
  /** A function receives the label's id, for the control's `aria-labelledby`. */
  children: React.ReactNode | ((labelId: string) => React.ReactNode);
}

/** Label (and description) on the left, control on the right; or stacked. */
export default function SettingRow({ label, desc, stacked, children }: Readonly<SettingRowProps>) {
  // hooks
  const labelId = useId();

  return (
    <div
      className={cx('border-base-content/8 flex gap-4 border-b py-2.5 last:border-none', {
        'items-center justify-between': !stacked,
        'flex-col gap-2.5 pb-3.5': stacked,
      })}
    >
      <div className="min-w-0">
        <div id={labelId} className="text-sm font-medium">
          {label}
        </div>
        {desc && <div className="text-muted mt-0.5 text-xs">{desc}</div>}
      </div>
      <div className="shrink-0">{typeof children === 'function' ? children(labelId) : children}</div>
    </div>
  );
}
