import { arrowStep } from '@/utils/radio';

import cx from 'classnames';

interface SegmentedProps<T extends string | number> {
  value: T;
  options: { value: T; label: string }[];
  onChange: (value: T) => void;
  /** Id of the element naming the group (a `SettingRow` label). */
  labelledBy: string;
  itemClassName?: string;
}

/** A pick-one choice of 2–4 equal-width options, with a thumb that slides to the selection. */
export default function Segmented<T extends string | number>({
  value,
  options,
  onChange,
  labelledBy,
  itemClassName,
}: Readonly<SegmentedProps<T>>) {
  const index = options.findIndex((option) => option.value === value);

  // handle
  const handleKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    const next = arrowStep(e.key, Math.max(index, 0), options.length);
    if (next === null) {
      return;
    }
    e.preventDefault();
    onChange(options[next].value);
    e.currentTarget.querySelectorAll<HTMLElement>('[role="radio"]')[next]?.focus();
  };

  return (
    <div
      role="radiogroup"
      aria-labelledby={labelledBy}
      className="bg-track relative grid auto-cols-fr grid-flow-col rounded-[10px] p-0.5"
      onKeyDown={handleKeyDown}
    >
      {index >= 0 && (
        // The thumb is one cell wide, so translating by 100% steps exactly one option.
        <span
          aria-hidden="true"
          className="bg-primary shadow-glass ease-spring absolute inset-y-0.5 left-0.5 rounded-lg transition-transform duration-450 motion-reduce:transition-none"
          style={{ width: `calc((100% - 4px) / ${options.length})`, transform: `translateX(${index * 100}%)` }}
        />
      )}
      {options.map((option, i) => (
        <button
          key={option.value}
          type="button"
          role="radio"
          aria-checked={i === index}
          tabIndex={i === index || (index < 0 && i === 0) ? 0 : -1}
          className={cx(
            'focus-visible:outline-primary relative rounded-lg px-2.5 py-1 text-xs font-semibold transition-colors duration-200 focus-visible:outline-2 focus-visible:outline-offset-2 motion-reduce:transition-none',
            i === index ? 'text-primary-content' : 'text-base-content/70 hover:text-base-content',
            itemClassName,
          )}
          onClick={() => onChange(option.value)}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}
