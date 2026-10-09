import { useCallback, useEffect, useRef, useState, type DragEvent } from "react";
import { useTranslation } from "react-i18next";
import { readText } from "@tauri-apps/plugin-clipboard-manager";
import { api } from "../lib/tauri";
import { useQueue } from "../lib/queue";
import { toolsToast, useTools } from "../lib/tools";
import { useSettings } from "../lib/settings";
import { showError, useToasts } from "../lib/toast";
import { extractUrl, isEditableTarget } from "../lib/paste";
import { JobRow } from "../components/JobRow";
import { Icon } from "../components/Icon";
import { OptionsSheet } from "../components/OptionsSheet";
import { DropZone } from "../components/DropZone";
import { StatusBar } from "../components/StatusBar";
import type { VideoInfo } from "../lib/types";

interface SheetState {
  url: string;
  info: VideoInfo | null;
}

/** 첫 실행 등 도구가 준비되지 않았을 때 상단 배너 대신 지속 토스트를 띄운다. */
function useToolsToast() {
  const { t } = useTranslation();
  const status = useTools((s) => s.status);
  const kind = toolsToast(status);
  const error = status?.error;
  useEffect(() => {
    const { push, dismissKey } = useToasts.getState();
    if (kind === "hidden") dismissKey("tools");
    else if (kind === "failed")
      push({
        key: "tools", kind: "error", sticky: true,
        text: t("app.toolsFailed", { error }),
        action: { label: t("app.retry"), run: () => void api.updateTools().catch(showError) },
      });
    else push({ key: "tools", kind: "progress", sticky: true, progress: null, text: t("app.toolsPreparing"), desc: t("app.toolsPreparingDesc") });
  }, [kind, error, t]);
}

export default function MainWindow() {
  const { t } = useTranslation();
  useToolsToast();
  const jobs = useQueue((s) => s.jobs);
  const [sheet, setSheet] = useState<SheetState | null>(null);
  const [dragging, setDragging] = useState(false);
  // WebKit 은 dragleave 의 relatedTarget 을 비워 주기도 해서, 자식 경계를 넘을 때마다 꺼졌다 켜졌다 한다.
  // 들어온 횟수를 세어 창 밖으로 완전히 나갔을 때만 끈다.
  const dragDepth = useRef(0);
  // 시트가 열려 있으면 ⌘V·드롭을 무시한다(열린 시트를 덮어쓰지 않게). keydown effect가 낡지 않도록 ref로 읽는다.
  const sheetUrl = useRef<string | null>(null);
  sheetUrl.current = sheet?.url ?? null;
  // 앞선 붙여넣기(readText·addUrl·probe)가 끝나기 전의 두 번째 요청은 무시한다(중복 작업 방지).
  const inFlight = useRef(false);

  const submit = useCallback(async (source: string | null | Promise<string | null>) => {
    if (inFlight.current) return;
    inFlight.current = true;
    let url: string | null;
    try {
      url = extractUrl(await source);
      if (!url) return showError({ code: "invalid_url" });
      if (!useTools.getState().status?.ready) return showError({ code: "ytdlp_missing" });
      if (useSettings.getState().settings?.download.skip_sheet) {
        return await api.addUrl(url).then(() => undefined, showError);
      }
      setSheet({ url, info: null });
      sheetUrl.current = url; // 다시 렌더되기 전부터 ⌘V·드롭을 막는다.
    } finally {
      // probe 동안에는 열린 시트가 재진입을 막는다. 시트를 취소하면 바로 다시 붙여넣을 수 있고,
      // 늦게 온 probe 결과는 아래 url 비교가 걸러 낸다. probe는 이 guard 밖에 두어 새 요청의 guard를 풀지 않는다.
      inFlight.current = false;
    }
    try {
      const info = await api.probe(url);
      setSheet((cur) => (cur?.url === url ? { url, info } : cur));
    } catch (e) {
      // 그사이 취소하고 다른 시트를 열었다면 그 시트는 건드리지 않고 오류도 띄우지 않는다.
      const current = sheetUrl.current === url;
      setSheet((cur) => (cur?.url === url ? null : cur));
      if (current) showError(e);
    }
  }, []);

  // ⌘V와 드롭존 클릭이 같이 쓴다. 시트가 열려 있으면 무시한다.
  const paste = useCallback(() => {
    if (sheetUrl.current !== null) return;
    submit(readText().catch(() => null));
  }, [submit]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!e.metaKey) return;
      if (e.key === ",") {
        e.preventDefault();
        api.openSettings().catch(showError);
      } else if (e.key.toLowerCase() === "v" && !isEditableTarget(e.target)) {
        if (sheetUrl.current !== null) return;
        e.preventDefault();
        if (e.repeat) return;
        paste();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [paste]);

  const onDrop = (e: DragEvent) => {
    e.preventDefault();
    dragDepth.current = 0;
    setDragging(false);
    if (sheetUrl.current !== null) return;
    submit(e.dataTransfer.getData("text/uri-list") || e.dataTransfer.getData("text/plain"));
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
        if (dragDepth.current === 0) setDragging(false);
      }}
      onDrop={onDrop}
    >
      <header className="flex h-[42px] shrink-0 items-center gap-1 px-4">
        <span className="text-[15px] font-extrabold tracking-[-0.5px]">kiri<span className="text-primary">.</span></span>
        <span className="flex-1" />
        {jobs.some((j) => j.state.kind === "completed") && (
          <button className="btn btn-ghost btn-sm btn-square" aria-label={t("app.clearCompleted")} title={t("app.clearCompleted")} onClick={() => api.clearCompleted().catch(showError)}><Icon name="clear" className="h-4 w-4" /></button>
        )}
        <button className="btn btn-ghost btn-sm btn-square" aria-label={t("app.settings")} title={t("app.settings")} onClick={() => api.openSettings().catch(showError)}><Icon name="settings" className="h-4 w-4" /></button>
      </header>
      <DropZone dragging={dragging} onClick={paste} />
      <main className="flex-1 overflow-y-auto pb-1.5">
        {jobs.length === 0 ? (
          <div className="flex h-full flex-col items-center justify-center gap-1.5 text-center">
            <span className="mb-1 grid h-10 w-10 place-items-center rounded-xl bg-base-200 text-fg-muted"><Icon name="download" /></span>
            <span className="text-sm font-semibold">{t("app.empty")}</span>
            <span className="text-xs text-fg-muted">{t("app.emptyDesc")}</span>
          </div>
        ) : (
          [...jobs].reverse().map((j) => <JobRow key={j.id} job={j} />)
        )}
      </main>
      <StatusBar />
      {sheet && <OptionsSheet url={sheet.url} info={sheet.info} onClose={() => setSheet(null)} />}
    </div>
  );
}
