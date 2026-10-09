import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { api } from "../../lib/tauri";
import { showError } from "../../lib/toast";
import type { CliStatus } from "../../lib/types";
import { Row } from "./Row";
import { Group } from "./Group";

const USAGE = `kiri status            # running jobs
kiri list              # whole queue
kiri add <url> [--quality 1080p] [--format mp4-h264] [--subs ko,en]
kiri stop <id>
kiri remove <id>
kiri transcode <file> --format mp4-hevc [--quality 720p] [--output <dir>]
kiri list --json       # machine-readable`;

export function CliTab() {
  const { t } = useTranslation();
  const [st, setSt] = useState<CliStatus | null>(null);
  const refresh = () => api.cliStatus().then(setSt).catch(showError);
  useEffect(() => {
    refresh();
  }, []);
  const run = (f: () => Promise<void>) => () => f().then(refresh).catch(showError);

  return (
    <>
      <Group>
        <Row label={t("settings.cli.status")} desc={st?.installed ? t("settings.cli.installed", { path: st.link }) : t("settings.cli.notInstalled")}>
          {st?.installed ? (
            <button className="btn btn-sm btn-outline" onClick={run(api.uninstallCli)}>{t("settings.cli.uninstall")}</button>
          ) : (
            <button className="btn btn-sm btn-primary" onClick={run(api.installCli)}>{t("settings.cli.install")}</button>
          )}
        </Row>
      </Group>
      {st?.socket_error && (
        <div role="alert" className="alert alert-warning mt-2 py-2 text-sm">{t("settings.cli.socketError", { error: st.socket_error })}</div>
      )}
      <div className="mt-3 mb-1 text-xs font-semibold text-fg-muted">{t("settings.cli.usage")}</div>
      <pre className="surface-card overflow-x-auto p-3 text-xs select-text">{USAGE}</pre>
    </>
  );
}
