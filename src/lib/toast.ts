import { create } from "zustand";
import i18next from "i18next";

export interface Toast {
  id: number;
  /** 같은 key는 새로 쌓지 않고 그 자리에서 바꾼다. */
  key?: string;
  kind: "error" | "info" | "progress";
  text: string;
  desc?: string;
  /** progress 종류: 0~1, null이면 끝을 모르는 진행 */
  progress?: number | null;
  action?: { label: string; run: () => void };
  /** true면 자동으로 닫지 않는다. */
  sticky?: boolean;
}
export type ToastInput = Omit<Toast, "id">;

interface ToastStore {
  toasts: Toast[];
  push: (t: ToastInput) => number;
  dismiss: (id: number) => void;
  dismissKey: (key: string) => void;
}

export const TOAST_MS = 5000;
let seq = 0;
const timers = new Map<number, ReturnType<typeof setTimeout>>();
const clearTimer = (id: number) => {
  clearTimeout(timers.get(id));
  timers.delete(id);
};

export const useToasts = create<ToastStore>((set, get) => ({
  toasts: [],
  push: (input) => {
    const prev = input.key ? get().toasts.find((x) => x.key === input.key) : undefined;
    const id = prev?.id ?? ++seq;
    const toast = { ...input, id };
    clearTimer(id);
    set({ toasts: prev ? get().toasts.map((x) => (x.id === id ? toast : x)) : [...get().toasts, toast] });
    if (!input.sticky) timers.set(id, setTimeout(() => get().dismiss(id), TOAST_MS));
    return id;
  },
  dismiss: (id) => {
    clearTimer(id);
    set({ toasts: get().toasts.filter((x) => x.id !== id) });
  },
  dismissKey: (key) => {
    const x = get().toasts.find((t) => t.key === key);
    if (x) get().dismiss(x.id);
  },
}));

type T = (key: string, opts?: { defaultValue?: string }) => string;

/** invoke 오류 {code,message}, Error, 문자열 무엇이든 사람이 읽을 한 문장으로. */
export function errorText(e: unknown, t: T = (k, o) => i18next.t(k, o) as string): string {
  if (typeof e === "object" && e !== null && "code" in e) {
    const { code, message } = e as { code: string; message?: string };
    if (code === "unknown" && message) return message;
    return t(`error.${code}`, { defaultValue: message || t("error.unknown") });
  }
  const msg = e instanceof Error ? e.message : String(e);
  return msg.startsWith("error.") ? t(msg, { defaultValue: msg }) : msg;
}

export const showError = (e: unknown) => useToasts.getState().push({ kind: "error", text: errorText(e) });
export const showInfo = (text: string) => useToasts.getState().push({ kind: "info", text });
