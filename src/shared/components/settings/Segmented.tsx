import cx from 'classnames';

interface SegmentedProps<T extends string | number> {
  value: T;
  options: { value: T; label: string }[];
  onChange: (value: T) => void;
  /** Id of the element naming the group (a `SettingRow` label). */
  labelledBy: string;
  itemClassName?: string;
}

/** A pick-one choice of 2–4 options, as a joined button group. */
export default function Segmented<T extends string | number>({
  value,
  options,
  onChange,
  labelledBy,
  itemClassName,
}: Readonly<SegmentedProps<T>>) {
  return (
    <div role="radiogroup" aria-labelledby={labelledBy} className="join">
      {options.map((option) => (
        <button
          key={option.value}
          type="button"
          role="radio"
          aria-checked={option.value === value}
          className={cx('btn btn-sm join-item', itemClassName, { 'btn-primary': option.value === value })}
          onClick={() => onChange(option.value)}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}
