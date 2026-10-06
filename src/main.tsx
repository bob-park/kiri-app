import ReactDOM from "react-dom/client";
import "./index.css";
import { applyTheme } from "./lib/theme";
import { initI18n, resolveLang } from "./lib/i18n";

applyTheme("system");
initI18n(resolveLang("system", navigator.language));
ReactDOM.createRoot(document.getElementById("root")!).render(<div className="p-6">kiri</div>);
