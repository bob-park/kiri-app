import cx from 'classnames';

interface SwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  /** Id of the element naming the switch (a `SettingRow` label). */
  labelledBy: string;
}

/** On/off toggle whose knob springs across. */
export default function Switch({ checked, onChange, labelledBy }: Readonly<SwitchProps>) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-labelledby={labelledBy}
      className={cx(
        'focus-visible:outline-primary relative flex h-[22px] w-[38px] items-center rounded-full p-[3px] transition-colors duration-200 focus-visible:outline-2 focus-visible:outline-offset-2 motion-reduce:transition-none',
        checked ? 'bg-primary' : 'bg-base-content/20',
      )}
      onClick={() => onChange(!checked)}
    >
      <span
        className={cx(
          'bg-primary-content shadow-keycap ease-spring h-4 w-4 rounded-full transition-transform duration-450 motion-reduce:transition-none',
          { 'translate-x-4': checked },
        )}
      />
    </button>
  );
}
