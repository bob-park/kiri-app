import { useId } from 'react';

interface SettingRowProps {
  label: string;
  desc?: string;
  /** A function receives the label's id, for the control's `aria-labelledby`. */
  children: React.ReactNode | ((labelId: string) => React.ReactNode);
}

/** Label (and description) on the left, control on the right. */
export default function SettingRow({ label, desc, children }: Readonly<SettingRowProps>) {
  // hooks
  const labelId = useId();

  return (
    <div className="border-base-300 flex items-center justify-between gap-4 border-b py-2.5 last:border-none">
      <div className="min-w-0">
        <div id={labelId} className="text-sm font-medium">
          {label}
        </div>
        {desc && <div className="text-fg-muted mt-0.5 text-xs">{desc}</div>}
      </div>
      <div className="shrink-0">{typeof children === 'function' ? children(labelId) : children}</div>
    </div>
  );
}
