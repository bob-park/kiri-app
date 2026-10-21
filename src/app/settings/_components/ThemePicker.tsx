import type { ThemePref } from '@/domain/settings/apis/settings.dto';
import { arrowStep } from '@/utils/radio';

import cx from 'classnames';

const THEMES: ThemePref[] = ['system', 'light', 'dark'];

// Thumbnail of a theme: a background and three bars.
const PREVIEW = {
  light: { bg: 'bg-preview-light', bar: 'bg-preview-light-bar ring-1 ring-preview-light-edge' },
  dark: { bg: 'bg-preview-dark', bar: 'bg-preview-dark-bar' },
};

const BARS = ['inset-x-2 top-2.5', 'top-[22px] right-6 left-2', 'top-[34px] right-4 left-2'];

function Preview({ theme, className }: Readonly<{ theme: 'light' | 'dark'; className?: string }>) {
  return (
    <span className={cx('absolute inset-0', PREVIEW[theme].bg, className)}>
      {BARS.map((pos) => (
        <span key={pos} className={cx('absolute h-[7px] rounded', pos, PREVIEW[theme].bar)} />
      ))}
    </span>
  );
}

const MOTION = 'ease-spring duration-450 motion-reduce:transition-none';

interface ThemePickerProps {
  value: ThemePref;
  labels: Record<ThemePref, string>;
  onChange: (value: ThemePref) => void;
  /** Id of the element naming the group (a `SettingRow` label). */
  labelledBy: string;
}

/** Theme choice as preview cards: the selected card lifts with a primary border and its dot springs in. */
export default function ThemePicker({ value, labels, onChange, labelledBy }: Readonly<ThemePickerProps>) {
  const index = THEMES.indexOf(value);

  // handle
  const handleKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    const next = arrowStep(e.key, Math.max(index, 0), THEMES.length);
    if (next === null) {
      return;
    }
    e.preventDefault();
    onChange(THEMES[next]);
    e.currentTarget.querySelectorAll<HTMLElement>('[role="radio"]')[next]?.focus();
  };

  return (
    <div role="radiogroup" aria-labelledby={labelledBy} className="grid grid-cols-3 gap-2" onKeyDown={handleKeyDown}>
      {THEMES.map((theme) => {
        const on = theme === value;
        return (
          <button
            key={theme}
            type="button"
            role="radio"
            aria-checked={on}
            tabIndex={on ? 0 : -1}
            className={cx(
              'bg-glass focus-visible:outline-primary flex flex-col gap-[7px] rounded-xl border-[1.5px] p-1.5 pb-2 text-left transition-[translate,border-color] focus-visible:outline-2 focus-visible:outline-offset-2',
              MOTION,
              on ? 'border-primary shadow-glass -translate-y-0.5' : 'hover:border-primary/40 border-transparent',
            )}
            onClick={() => onChange(theme)}
          >
            <span
              aria-hidden="true"
              className="ring-base-content/8 relative h-[52px] overflow-hidden rounded-lg ring-1"
            >
              {/* System: the dark thumbnail over the light one, cut along a diagonal. */}
              <Preview theme={theme === 'dark' ? 'dark' : 'light'} />
              {theme === 'system' && (
                <Preview theme="dark" className="[clip-path:polygon(58%_0,100%_0,100%_100%,42%_100%)]" />
              )}
            </span>
            <span className="flex items-center gap-1.5 px-0.5 text-[11.5px] font-semibold">
              <span
                className={cx(
                  'grid h-4 w-4 shrink-0 place-items-center rounded-full border-[1.5px] transition-colors',
                  MOTION,
                  on ? 'border-primary' : 'border-base-content/25',
                )}
              >
                <span
                  className={cx('bg-primary h-2 w-2 rounded-full transition-transform', MOTION, {
                    'scale-0': !on,
                  })}
                />
              </span>
              {labels[theme]}
            </span>
          </button>
        );
      })}
    </div>
  );
}
