import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "../components/Icon";
import { GeneralTab } from "./settings/GeneralTab";
import { DownloadTab } from "./settings/DownloadTab";
import { CliTab } from "./settings/CliTab";
import { UpdateTab } from "./settings/UpdateTab";

const TABS = [
  { key: "general", icon: "settings", Body: GeneralTab },
  { key: "download", icon: "download", Body: DownloadTab },
  { key: "cli", icon: "terminal", Body: CliTab },
  { key: "update", icon: "refresh", Body: UpdateTab },
] as const;

export default function SettingsWindow() {
  const { t } = useTranslation();
  const [tab, setTab] = useState<(typeof TABS)[number]["key"]>("general");
  const Body = TABS.find((x) => x.key === tab)!.Body;
  return (
    <div className="flex h-full flex-col">
      <nav role="tablist" className="flex justify-center gap-1 border-b border-base-300 bg-base-200 p-2">
        {TABS.map((x) => (
          <button
            key={x.key}
            role="tab"
            aria-selected={tab === x.key}
            className={`flex w-20 flex-col items-center gap-0.5 rounded-xl px-2 py-1 text-xs ${tab === x.key ? "bg-secondary font-semibold text-secondary-content" : "text-fg-muted"}`}
            onClick={() => setTab(x.key)}
          >
            <Icon name={x.icon} className="h-5 w-5" />
            {t(`settings.tab.${x.key}`)}
          </button>
        ))}
      </nav>
      <div className="flex-1 overflow-y-auto px-5 py-3">
        <Body />
      </div>
    </div>
  );
}
