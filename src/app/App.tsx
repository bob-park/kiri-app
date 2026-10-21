import { useEffect, useRef, useState } from 'react';

import { useQueryClient } from '@tanstack/react-query';

import DropZone from '@/app/_components/DropZone';
import { probe } from '@/domain/queue/apis/queue';
import type { VideoInfo } from '@/domain/queue/apis/queue.dto';
import JobRow from '@/domain/queue/components/JobRow';
import OptionsSheet from '@/domain/queue/components/OptionsSheet';
import { useCompletedClear, useJobs, useUrlAdd } from '@/domain/queue/queries/queue';
import { openSettings } from '@/domain/settings/apis/settings';
import type { Settings } from '@/domain/settings/apis/settings.dto';
import { SETTINGS_QUERY_KEY, useSettings } from '@/domain/settings/queries/settings';
import type { ToolsStatus } from '@/domain/tools/apis/tools.dto';
import { TOOLS_QUERY_KEY, useToolsStatus, useToolsUpdate } from '@/domain/tools/queries/tools';
import type { AppError } from '@/shared/api/common.dto';
import Icon from '@/shared/components/Icon';
import StatusBar from '@/shared/components/StatusBar';
import useI18n from '@/shared/hooks/useI18n';
import { useStore } from '@/shared/store/rootStore';
import { extractUrl, isEditableTarget } from '@/utils/paste';
import { toolsToast } from '@/utils/tools';

import { readText } from '@tauri-apps/plugin-clipboard-manager';

interface SheetState {
  url: string;
  info: VideoInfo | null;
}

/** The `main` window: paste a link, pick options, watch the queue. */
export default function App() {
  // ref
  // While a sheet is open, ⌘V and drops are ignored (an open sheet is never replaced).
  // Read through a ref so the keydown listener never sees a stale value.
  const sheetUrl = useRef<string | null>(null);
  // A second paste before the first (readText · addUrl) finished is dropped (no duplicate jobs).
  const inFlight = useRef(false);
  // WebKit sometimes clears dragleave's relatedTarget, so crossing a child toggles the highlight.
  // Count enters and switch off only when the pointer has left the window.
  const dragDepth = useRef(0);

  // state
  const [sheet, setSheet] = useState<SheetState | null>(null);
  const [dragging, setDragging] = useState(false);

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
  const { updateTools } = useToolsUpdate();
  const { addUrl } = useUrlAdd();
  const { clearCompleted } = useCompletedClear();

  sheetUrl.current = sheet?.url ?? null;
  const toolsState = toolsToast(tools);
  const toolsError = tools?.error ?? '';

  // useEffect
  // Until the tools are ready (e.g. first launch), a sticky toast says so.
  useEffect(() => {
    if (toolsState === 'hidden') {
      dismissToastByKey('tools');
    } else if (toolsState === 'failed') {
      pushToast({
        key: 'tools',
        kind: 'error',
        sticky: true,
        text: t.app.toolsFailed(toolsError),
        action: { label: t.app.retry, run: () => updateTools() },
      });
    } else {
      pushToast({
        key: 'tools',
        kind: 'progress',
        sticky: true,
        progress: null,
        text: t.app.toolsPreparing,
        desc: t.app.toolsPreparingDesc,
      });
    }
  }, [toolsState, toolsError, t]);

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

  const handleSubmit = async (source: string | null | Promise<string | null>) => {
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
      if (!queryClient.getQueryData<ToolsStatus>(TOOLS_QUERY_KEY)?.ready) {
        return showError({ code: 'YTDLP_MISSING', message: '' });
      }
      if (queryClient.getQueryData<Settings>(SETTINGS_QUERY_KEY)?.download.skip_sheet) {
        // The mutation cache shows a failure.
        await addUrl(url).catch(() => undefined);

        return;
      }
      setSheet({ url, info: null });
      sheetUrl.current = url; // block ⌘V and drops before the next render
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
      }
    }
  };

  // ⌘V and a click on the drop zone. Ignored while a sheet is open.
  const handlePaste = () => {
    if (sheetUrl.current !== null) {
      return;
    }
    void handleSubmit(readText().catch(() => null));
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    dragDepth.current = 0;
    setDragging(false);
    if (sheetUrl.current !== null) {
      return;
    }
    void handleSubmit(e.dataTransfer.getData('text/uri-list') || e.dataTransfer.getData('text/plain'));
  };

  return (
    <div
      className="flex h-full flex-col"
      onDragOver={(e) => e.preventDefault()}
      onDragEnter={() => {
        dragDepth.current += 1;
        setDragging(true);
      }}
      onDragLeave={() => {
        dragDepth.current = Math.max(0, dragDepth.current - 1);
        if (dragDepth.current === 0) {
          setDragging(false);
        }
      }}
      onDrop={handleDrop}
    >
      <header className="flex h-[42px] shrink-0 items-center gap-1 px-4">
        <span className="text-[15px] font-extrabold tracking-[-0.5px]">
          kiri<span className="text-primary">.</span>
        </span>
        <span className="flex-1" />
        {jobs.some((j) => j.state.kind === 'completed') && (
          <button
            className="btn btn-ghost btn-sm btn-square"
            aria-label={t.app.clearCompleted}
            title={t.app.clearCompleted}
            onClick={() => clearCompleted()}
          >
            <Icon name="clear" className="h-4 w-4" />
          </button>
        )}
        <button
          className="btn btn-ghost btn-sm btn-square"
          aria-label={t.app.settings}
          title={t.app.settings}
          onClick={handleOpenSettings}
        >
          <Icon name="settings" className="h-4 w-4" />
        </button>
      </header>
      <DropZone dragging={dragging} onClick={handlePaste} />
      <main className="flex-1 overflow-y-auto pb-1.5">
        {jobs.length === 0 ? (
          <div className="flex h-full flex-col items-center justify-center gap-1.5 text-center">
            <span className="bg-base-200 text-fg-muted mb-1 grid h-10 w-10 place-items-center rounded-xl">
              <Icon name="download" />
            </span>
            <span className="text-sm font-semibold">{t.app.empty}</span>
            <span className="text-fg-muted text-xs">{t.app.emptyDesc}</span>
          </div>
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
