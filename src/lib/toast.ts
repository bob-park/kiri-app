import { create } from "zustand";
import i18next from "i18next";

export interface Toast {
  id: number;
  kind: "error" | "info";
  text: string;
}

interface ToastStore {
  toasts: Toast[];
  push: (kind: Toast["kind"], text: string) => void;
  dismiss: (id: number) => void;
}

let seq = 0;

export const useToasts = create<ToastStore>((set, get) => ({
  toasts: [],
  push: (kind, text) => {
    const id = ++seq;
    set({ toasts: [...get().toasts, { id, kind, text }] });
    setTimeout(() => get().dismiss(id), 5000);
  },
  dismiss: (id) => set({ toasts: get().toasts.filter((t) => t.id !== id) }),
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

export const showError = (e: unknown) => useToasts.getState().push("error", errorText(e));
export const showInfo = (text: string) => useToasts.getState().push("info", text);
