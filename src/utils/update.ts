// Type-only import: erased at runtime, so node --test needs no alias resolution.
import type { UpdateInfo, UpdateProgress, UpdateStatus } from '@/domain/updater/apis/updater.dto';

export type UpdatePhase = 'idle' | 'downloading' | 'ready' | 'failed' | 'scheduled' | 'installing';

export type UpdateEvent =
  | { type: 'found'; info: UpdateInfo }
  | { type: 'progress' }
  | { type: 'ready' }
  | { type: 'downloadFailed' }
  | { type: 'retry' }
  | { type: 'scheduled' }
  | { type: 'install' }
  | { type: 'installFailed'; ready: boolean };

/** An install flow the user chose is never overridden by background events. */
const userDriven = (p: UpdatePhase) => p === 'scheduled' || p === 'installing';

export function nextPhase(p: UpdatePhase, e: UpdateEvent): UpdatePhase {
  switch (e.type) {
    case 'found':
      if (userDriven(p)) {
        return p;
      }

      return e.info.ready ? 'ready' : e.info.downloading ? 'downloading' : 'failed';
    case 'progress':
      return userDriven(p) ? p : 'downloading';
    case 'ready':
      return userDriven(p) ? p : 'ready';
    case 'downloadFailed':
      return userDriven(p) ? p : 'failed';
    case 'retry':
      return 'downloading';
    case 'scheduled':
      return 'scheduled';
    case 'install':
      return 'installing';
    case 'installFailed':
      // Nothing to install (e.g. a failed check): stay idle.
      if (p === 'idle') {
        return p;
      }

      return e.ready ? 'ready' : 'failed';
  }
}

export const updatePct = (p: UpdateProgress | null) => (p?.total ? Math.round((p.received / p.total) * 100) : null);

/** The window's view of the app update, cached under `['updater','status']`. */
export interface UpdateView {
  info: UpdateInfo | null;
  phase: UpdatePhase;
  progress: UpdateProgress | null;
  /** A manual check finished; only then does Settings say "up to date". */
  checked: boolean;
}

export const INITIAL_UPDATE_VIEW: UpdateView = { info: null, phase: 'idle', progress: null, checked: false };

export const stepUpdate = (v: UpdateView, e: UpdateEvent): UpdateView => ({ ...v, phase: nextPhase(v.phase, e) });

export const foundUpdate = (v: UpdateView, info: UpdateInfo): UpdateView =>
  stepUpdate({ ...v, info, progress: null }, { type: 'found', info });

/** A second install request while one runs. Not worth telling the user. */
export const isBusy = (message: string) => message === 'busy';

const infoOf = (s: UpdateInfo): UpdateInfo => ({
  version: s.version,
  notes: s.notes,
  ready: s.ready,
  downloading: s.downloading,
});

/**
 * Applies an `update_status` snapshot. It fills in a missing update or moves a download forward
 * to ready, but never rolls back a phase that newer events (or the user) already set.
 */
export function applyFetchedUpdate(v: UpdateView, info: UpdateInfo | null): UpdateView {
  if (!info) {
    return v;
  }
  if (!v.info) {
    return foundUpdate(v, info);
  }
  if (info.ready && !v.info.ready) {
    return stepUpdate({ ...v, info, progress: null }, { type: 'ready' });
  }

  return v;
}

/** Applies one `update://status` event. */
export function applyUpdateStatus(v: UpdateView, s: UpdateStatus): UpdateView {
  switch (s.kind) {
    case 'available':
      return foundUpdate(v, infoOf(s));
    case 'progress':
      return stepUpdate({ ...v, progress: { received: s.received, total: s.total } }, { type: 'progress' });
    case 'ready':
      return stepUpdate({ ...v, info: infoOf(s), progress: null }, { type: 'ready' });
    case 'download_failed':
      return stepUpdate(v, { type: 'downloadFailed' });
    case 'scheduled':
      return stepUpdate(v, { type: 'scheduled' });
    case 'error':
      return isBusy(s.error) ? v : stepUpdate(v, { type: 'installFailed', ready: v.info?.ready ?? false });
  }
}
