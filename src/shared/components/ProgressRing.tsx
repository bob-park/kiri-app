interface ProgressRingProps {
  /** 0..1 */
  ratio: number;
  label: string;
}

/** A small determinate ring with the percentage inside: brand stroke over the track color. */
export default function ProgressRing({ ratio, label }: Readonly<ProgressRingProps>) {
  const percent = Math.round(ratio * 100);

  return (
    <span
      role="progressbar"
      aria-label={label}
      aria-valuenow={percent}
      aria-valuemin={0}
      aria-valuemax={100}
      className="relative grid h-[30px] w-[30px] shrink-0 place-items-center"
    >
      <svg viewBox="0 0 30 30" className="absolute inset-0 -rotate-90" aria-hidden>
        <circle cx="15" cy="15" r="13" fill="none" strokeWidth="3" className="stroke-track" />
        <circle
          cx="15"
          cy="15"
          r="13"
          fill="none"
          strokeWidth="3"
          strokeLinecap="round"
          pathLength={100}
          strokeDasharray={`${percent} 100`}
          className="stroke-primary"
        />
      </svg>
      <span className="text-primary relative text-[8.5px] font-bold tabular-nums">{percent}%</span>
    </span>
  );
}
