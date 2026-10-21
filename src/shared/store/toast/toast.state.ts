export interface Toast {
  id: number;
  /** A toast with the same key is replaced in place instead of stacking. */
  key?: string;
  kind: 'error' | 'info' | 'progress';
  text: string;
  desc?: string;
  /** For `progress`: 0..1, or `null` when the end is unknown. */
  progress?: number | null;
  action?: { label: string; run: () => void };
  /** Never auto-dismissed when true. */
  sticky?: boolean;
}

export type ToastInput = Omit<Toast, 'id'>;

export type ShownToast = Toast & { leaving?: boolean };

export interface ToastState {
  toasts: Toast[];
  /** Shows (or replaces, by `key`) a toast and returns its id. */
  pushToast: (input: ToastInput) => number;
  dismissToast: (id: number) => void;
  dismissToastByKey: (key: string) => void;
}
