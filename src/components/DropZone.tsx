import { useTranslation } from "react-i18next";
import { Icon } from "./Icon";

/** 붙여넣기·드롭 안내. 누르면 클립보드에서 붙여넣는다(⌘V와 같음). */
export function DropZone({ dragging, onClick }: { dragging: boolean; onClick: () => void }) {
  const { t } = useTranslation();
  return (
    <button
      type="button"
      onClick={onClick}
      className={`mx-3.5 mt-1.5 mb-2.5 flex items-center gap-3 rounded-[14px] border-[1.5px] border-dashed px-3 py-2.5 text-left transition-colors ${
        dragging ? "border-primary bg-secondary" : "border-primary/40 bg-primary/5 hover:bg-primary/10"
      }`}
    >
      <span className="grid h-[30px] w-[30px] shrink-0 place-items-center rounded-[10px] bg-secondary text-primary">
        <Icon name="download" className="h-4 w-4" />
      </span>
      <span className="min-w-0 flex-1">
        <span className="block truncate text-[12.5px] font-semibold">{t("app.dropTitle")}</span>
        <span className="mt-0.5 block text-[11px] text-fg-muted">
          <kbd className="kbd kbd-xs">⌘</kbd> <kbd className="kbd kbd-xs">V</kbd> {t("app.dropShortcut")}
        </span>
      </span>
    </button>
  );
}
