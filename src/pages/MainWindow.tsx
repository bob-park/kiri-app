import { useCallback, useEffect, useRef, useState, type DragEvent } from "react";
import { useTranslation } from "react-i18next";
import { readText } from "@tauri-apps/plugin-clipboard-manager";
import { api } from "../lib/tauri";
import { useQueue } from "../lib/queue";
import { useTools } from "../lib/tools";
import { useSettings } from "../lib/settings";
import { showError } from "../lib/toast";
import { extractUrl, isEditableTarget } from "../lib/paste";
import { JobRow } from "../components/JobRow";
import { Icon } from "../components/Icon";
import { OptionsSheet } from "../components/OptionsSheet";
import { UpdateBanner } from "../components/UpdateBanner";
import type { VideoInfo } from "../lib/types";

interface SheetState {
  url: string;
  info: VideoInfo | null;
}

function ToolsNotice() {
  const { t } = useTranslation();
  const status = useTools((s) => s.status);
  if (status?.ready) return null;
  const failed = status?.error && !status.installing;
  if (failed) {
    return (
      <div role="status" className="mx-3 mt-2 flex items-center gap-2 rounded-xl bg-error/15 px-3 py-2 text-sm text-error">
        <span className="flex-1">{t("app.toolsFailed", { error: status!.error })}</span>
        <button className="btn btn-xs" onClick={() => api.updateTools().catch(showError)}>{t("app.retry")}</button>
      </div>
    );
  }
  return (
    <div role="status" aria-live="polite" className="mx-3 mt-2 rounded-xl bg-secondary px-3 py-2.5 text-secondary-content">
      <div className="flex items-center gap-2 text-sm font-semibold">
        <span className="loading loading-spinner loading-sm" />
        {t("app.toolsPreparing")}
      </div>
      <p className="mt-1 text-xs">{t("app.toolsPreparingDesc")}</p>
      <progress className="progress progress-primary mt-2 h-1.5 w-full" aria-label={t("app.toolsPreparing")} />
    </div>
  );
}

export default function MainWindow() {
  const { t } = useTranslation();
  const jobs = useQueue((s) => s.jobs);
  const [sheet, setSheet] = useState<SheetState | null>(null);
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
        submit(readText().catch(() => null));
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [submit]);

  const onDrop = (e: DragEvent) => {
    e.preventDefault();
    if (sheetUrl.current !== null) return;
    submit(e.dataTransfer.getData("text/uri-list") || e.dataTransfer.getData("text/plain"));
  };

  return (
    <div className="flex h-full flex-col" onDragOver={(e) => e.preventDefault()} onDrop={onDrop}>
      <header className="flex items-center gap-2 border-b border-base-300 bg-base-200 px-4 py-2">
        <span className="font-bold tracking-tight">kiri</span>
        <span className="flex-1 truncate text-center text-xs text-fg-muted">{t("app.dropHint")}</span>
        <button className="btn btn-ghost btn-sm btn-square" aria-label={t("app.settings")} title={t("app.settings")} onClick={() => api.openSettings().catch(showError)}><Icon name="settings" /></button>
      </header>
      <UpdateBanner />
      <ToolsNotice />
      <main className="flex-1 overflow-y-auto">
        {jobs.length === 0 ? (
          <div className="grid h-full place-items-center text-sm text-fg-muted">{t("app.empty")}</div>
        ) : (
          [...jobs].reverse().map((j) => <JobRow key={j.id} job={j} />)
        )}
      </main>
      {sheet && <OptionsSheet url={sheet.url} info={sheet.info} onClose={() => setSheet(null)} />}
    </div>
  );
}
