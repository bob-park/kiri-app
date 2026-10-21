interface SwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  /** Id of the element naming the switch (a `SettingRow` label). */
  labelledBy: string;
}

export default function Switch({ checked, onChange, labelledBy }: Readonly<SwitchProps>) {
  return (
    <input
      type="checkbox"
      role="switch"
      aria-labelledby={labelledBy}
      className="toggle toggle-primary toggle-sm"
      checked={checked}
      onChange={(e) => onChange(e.target.checked)}
    />
  );
}
