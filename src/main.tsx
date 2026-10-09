import React, { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./index.css";
import { useSettings } from "./lib/settings";
import { useQueue } from "./lib/queue";
import { useTools } from "./lib/tools";
import { useUpdate } from "./lib/update";
import { applyTheme } from "./lib/theme";
import { initI18n, resolveLang } from "./lib/i18n";
import { Toasts } from "./components/Toasts";

const MainWindow = React.lazy(() => import("./pages/MainWindow"));
const SettingsWindow = React.lazy(() => import("./pages/SettingsWindow"));
const label = getCurrentWindow().label;

function Root() {
  const { settings, load, subscribeBackend } = useSettings();
  const [ready, setReady] = useState(false);

  useEffect(() => {
    const unsubs = [subscribeBackend(), useQueue.getState().bind(), useTools.getState().bind(), useUpdate.getState().subscribe()];
    load().then(() => setReady(true));
    return () => unsubs.forEach((u) => u());
  }, []);

  useEffect(() => {
    if (!settings) return;
    applyTheme(settings.general.theme);
    initI18n(resolveLang(settings.general.ui_language, navigator.language));
  }, [settings?.general.theme, settings?.general.ui_language]);

  if (!ready || !settings) return null;
  return (
    <React.Suspense fallback={null}>
      {label === "main" && <MainWindow />}
      {label === "settings" && <SettingsWindow />}
      <Toasts bottom={label === "main" ? 44 : 16} />
    </React.Suspense>
  );
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <Root />
  </React.StrictMode>,
);
