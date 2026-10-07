import type { Job } from "./types";

type T = (key: string, opts?: Record<string, unknown>) => string;

export function formatBytes(n: number | null | undefined): string {
  if (!n) return "";
  const units = ["B", "KB", "MB", "GB"];
  let v = n;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return i === 0 ? `${v} B` : `${v.toFixed(1)} ${units[i]}`;
}

export function formatDuration(secs: number | null | undefined): string {
  if (secs == null) return "";
  const s = Math.round(secs);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const r = String(s % 60).padStart(2, "0");
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${r}` : `${m}:${r}`;
}

export const pct = (p: number) => `${Math.round(p * 100)}%`;

/** "error." 으로 시작하면 i18n 키, 아니면 도구 원문. */
export function failureText(message: string, t: T): string {
  return message.startsWith("error.") ? t(message, { defaultValue: message }) : message;
}

/** 행의 둘째 줄: 화질 · 포맷 · 자막 · 상태별 정보 */
export function jobDetail(job: Job, t: T): string {
  const parts = [["audio", "original"].includes(job.quality_label) ? t(`quality.${job.quality_label}`) : job.quality_label, t(`preset.${job.options.preset}`)];
  if (job.source.kind === "file") parts.unshift(t("job.transcode"));
  if (job.options.subtitles.length) parts.push(t("job.subs", { langs: job.options.subtitles.join(", ") }));
  const s = job.state;
  if (s.kind === "downloading") parts.push([pct(job.progress), job.speed, job.eta].filter(Boolean).join(" · "));
  else if (s.kind === "encoding") parts.push(`${t("state.encoding")} ${pct(job.progress)}`);
  else if (s.kind === "failed") parts.push(failureText(s.message, t));
  return parts.join(" · ");
}
