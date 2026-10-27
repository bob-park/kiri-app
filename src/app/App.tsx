import { useEffect, useRef, useState } from 'react';

import { useQueryClient } from '@tanstack/react-query';

import { probe } from '@/domain/queue/apis/queue';
import type { VideoInfo } from '@/domain/queue/apis/queue.dto';
import EmptyState from '@/domain/queue/components/EmptyState';
import JobRow from '@/domain/queue/components/JobRow';
import OptionsSheet from '@/domain/queue/components/OptionsSheet';
import { useCompletedClear, useJobs, useUrlAdd } from '@/domain/queue/queries/queue';
import { openSettings } from '@/domain/settings/apis/settings';
import type { Settings } from '@/domain/settings/apis/settings.dto';
import { SETTINGS_QUERY_KEY, useSettings } from '@/domain/settings/queries/settings';
import { checkYtdlpUpdate } from '@/domain/tools/apis/tools';
import type { ToolsStatus } from '@/domain/tools/apis/tools.dto';
import { TOOLS_QUERY_KEY, useToolsInstall, useToolsStatus, useToolsUpdate } from '@/domain/tools/queries/tools';
import type { AppError } from '@/shared/api/common.dto';
import Icon from '@/shared/components/Icon';
import StatusBar from '@/shared/components/StatusBar';
import useI18n from '@/shared/hooks/useI18n';
import { useStore } from '@/shared/store/rootStore';
import { extractUrl, isEditableTarget } from '@/utils/paste';
import { newlyFailedYoutube, toolsToast, updateOutcome } from '@/utils/tools';

import { readText } from '@tauri-apps/plugin-clipboard-manager';

const HEADER_BUTTON =
  'glass text-muted hover:text-base-content focus-visible:outline-primary grid h-7 w-7 cursor-pointer place-items-center rounded-[9px] focus-visible:outline-2';

interface SheetState {
  url: string;
  info: VideoInfo | null;
}

/** The `main` window: paste a link, pick options, watch the queue. */
export default function App() {
  // ref
  // While a sheet is open, ⌘V is ignored (an open sheet is never replaced).
  // Read through a ref so the keydown listener never sees a stale value.
  const sheetUrl = useRef<string | null>(null);
  // A second paste before the first (readText · addUrl) finished is dropped (no duplicate jobs).
  const inFlight = useRef(false);
  // The yt-dlp version already hinted this session (once per latest version).
  const hinted = useRef<string | null>(null);
  // Each job's last seen state kind, to spot a youtube job failing now (not one restored failed).
  const seen = useRef<ReadonlyMap<number, string>>(new Map());
  // The first render's keydown closure reaches `maybeHintYtdlp`; it reads the current language here.
  const latestT = useRef<typeof t | null>(null);

  // state
  const [sheet, setSheet] = useState<SheetState | null>(null);
  const [pressCount, setPressCount] = useState(0);

  // store
  const showError = useStore((s) => s.showError);
  const pushToast = useStore((s) => s.pushToast);
  const dismissToastByKey = useStore((s) => s.dismissToastByKey);

  // hooks
  const { t } = useI18n();
  const queryClient = useQueryClient();

  // queries
  const { settings } = useSettings();
  const { jobs } = useJobs();
  const { status: tools } = useToolsStatus();
  const { installTools } = useToolsInstall();
  const { updateTools } = useToolsUpdate();
  const { addUrl } = useUrlAdd();
  const { clearCompleted } = useCompletedClear();

  sheetUrl.current = sheet?.url ?? null;
  latestT.current = t;
  const toolsState = toolsToast(tools);
  const toolsError = tools?.error ?? '';
  const toolsInstalling = tools?.installing ?? '';

  // useEffect
  // While a missing tool downloads (e.g. first launch), a sticky toast names it.
  useEffect(() => {
    if (toolsState === 'hidden') {
      dismissToastByKey('tools');
    } else if (toolsState === 'failed') {
      pushToast({
        key: 'tools',
        kind: 'error',
        sticky: true,
        text: t.app.toolsFailed(toolsError),
        action: { label: t.app.retry, run: () => installTools() },
      });
    } else {
      pushToast({
        key: 'tools',
        kind: 'progress',
        sticky: true,
        progress: null,
        text: t.app.toolsInstalling(toolsInstalling),
        desc: t.app.toolsPreparingDesc,
      });
    }
  }, [toolsState, toolsError, toolsInstalling, t]);

  // A youtube job failing now may be fixed by a newer yt-dlp.
  useEffect(() => {
    const r = newlyFailedYoutube(seen.current, jobs);
    seen.current = r.seen;
    if (r.failed) {
      void maybeHintYtdlp();
    }
  }, [jobs]);

  // The handlers below only use refs, state setters and stable hook results, so the first render's are enough.
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!e.metaKey) {
        return;
      }
      if (e.key === ',') {
        e.preventDefault();
        handleOpenSettings();
      } else if (e.key.toLowerCase() === 'v' && !isEditableTarget(e.target)) {
        if (sheetUrl.current !== null) {
          return;
        }
        e.preventDefault();
        if (!e.repeat) {
          handlePaste();
        }
      }
    };
    window.addEventListener('keydown', handleKeyDown);

    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  // handle
  const handleOpenSettings = () => void openSettings().catch((e: AppError) => showError(e));

  const handleSubmit = async (source: Promise<string | null>) => {
    if (inFlight.current) {
      return;
    }
    inFlight.current = true;
    let url: string | null;
    try {
      url = extractUrl(await source);
      if (!url) {
        return showError({ code: 'INVALID_URL', message: '' });
      }
      if (!navigator.onLine) {
        return showError({ code: 'OFFLINE', message: '' });
      }
      if (!queryClient.getQueryData<ToolsStatus>(TOOLS_QUERY_KEY)?.ready) {
        return showError({ code: 'YTDLP_MISSING', message: '' });
      }
      if (queryClient.getQueryData<Settings>(SETTINGS_QUERY_KEY)?.download.skip_sheet) {
        // The mutation cache shows a failure.
        await addUrl(url).catch(() => undefined);

        return;
      }
      setSheet({ url, info: null });
      sheetUrl.current = url; // block ⌘V before the next render
    } finally {
      // While probing, the open sheet blocks re-entry. Cancelling the sheet allows a new paste at once;
      // a late probe result is filtered by the url check below. The probe stays outside this guard
      // so it never releases a newer request's guard.
      inFlight.current = false;
    }
    try {
      const info = await probe(url);
      setSheet((cur) => (cur?.url === url ? { url, info } : cur));
    } catch (e) {
      // Cancelled and opened another sheet meanwhile: leave that one alone and stay quiet.
      const current = sheetUrl.current === url;
      setSheet((cur) => (cur?.url === url ? null : cur));
      if (current) {
        showError(e as AppError);
        void maybeHintYtdlp();
      }
    }
  };

  // Uses only refs and stable store/mutation functions, so a first-render closure is safe.
  const maybeHintYtdlp = async () => {
    const version = await checkYtdlpUpdate().catch(() => null);
    const tr = latestT.current;
    if (!version || version === hinted.current || !tr) {
      return;
    }
    pushToast({
      key: 'ytdlp-hint',
      kind: 'info',
      text: tr.app.ytdlpHintTitle,
      desc: tr.app.ytdlpHintDesc,
      action: { label: tr.settings.update.update, run: handleHintUpdate },
    });
    hinted.current = version;
  };

  // The hint's Update: refused offline, else the hint becomes a progress toast, then the result.
  // Like `maybeHintYtdlp`, it runs from a first-render closure, so it reads only refs and stable functions.
  const handleHintUpdate = () => {
    const tr = latestT.current;
    if (!tr) {
      return;
    }
    if (!navigator.onLine) {
      return showError({ code: 'OFFLINE', message: '' });
    }
    pushToast({
      key: 'ytdlp-hint',
      kind: 'progress',
      sticky: true,
      progress: null,
      text: tr.app.toolsInstalling('yt-dlp'),
    });
    updateTools(['yt-dlp'], {
      onSuccess: (status) => {
        const cur = latestT.current ?? tr; // the language may have changed meanwhile
        const outcome = updateOutcome(status);
        if (outcome.kind === 'failed') {
          pushToast({ key: 'ytdlp-hint', kind: 'error', text: cur.app.ytdlpUpdateFailed(outcome.error) });
        } else {
          pushToast({
            key: 'ytdlp-hint',
            kind: 'info',
            text: outcome.kind === 'updated' ? cur.app.ytdlpUpdated : cur.app.toolsBusy,
          });
        }
      },
      // ErrorAlert shows the error.
      onError: () => dismissToastByKey('ytdlp-hint'),
    });
  };

  // ⌘V. Ignored while a sheet is open.
  const handlePaste = () => {
    if (sheetUrl.current !== null) {
      return;
    }
    setPressCount((n) => n + 1);
    void handleSubmit(readText().catch(() => null));
  };

  return (
    <div className="relative flex h-full flex-col">
      <div className="aurora" aria-hidden="true">
        <span className="aurora-blob" />
        <span className="aurora-blob" />
        <span className="aurora-blob" />
      </div>
      <header className="relative flex h-[46px] shrink-0 items-center gap-1.5 px-4">
        <span className="text-[17px] font-extrabold tracking-[-0.5px]">
          kiri<span className="text-primary">.</span>
        </span>
        <span className="flex-1" />
        {jobs.some((j) => j.state.kind === 'completed') && (
          <button
            className={HEADER_BUTTON}
            aria-label={t.app.clearCompleted}
            title={t.app.clearCompleted}
            onClick={() => clearCompleted()}
          >
            <Icon name="clear" className="h-4 w-4" />
          </button>
        )}
        <button
          className={`group ${HEADER_BUTTON}`}
          aria-label={t.app.settings}
          title={t.app.settings}
          onClick={handleOpenSettings}
        >
          <Icon
            name="settings"
            className="h-4 w-4 transition-transform duration-300 motion-safe:group-hover:rotate-45 motion-reduce:transition-none"
          />
        </button>
      </header>
      <main className="relative flex-1 overflow-y-auto pt-1 pb-1.5">
        {jobs.length === 0 ? (
          <EmptyState pressCount={pressCount} />
        ) : (
          [...jobs].reverse().map((j) => <JobRow key={j.id} job={j} />)
        )}
      </main>
      <StatusBar jobs={jobs} />
      {sheet && (
        <OptionsSheet url={sheet.url} info={sheet.info} defaults={settings.download} onClose={() => setSheet(null)} />
      )}
    </div>
  );
}
