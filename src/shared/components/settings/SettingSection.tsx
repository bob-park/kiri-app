interface SettingSectionProps {
  title?: string;
  children: React.ReactNode;
}

/** A glass card of `SettingRow`s with an optional small heading above it. */
export default function SettingSection({ title, children }: Readonly<SettingSectionProps>) {
  return (
    <section className="mb-3">
      {title && <h3 className="text-muted mx-1 mb-1.5 text-[10.5px] font-bold">{title}</h3>}
      <div className="glass shadow-glass rounded-2xl px-3.5">{children}</div>
    </section>
  );
}
