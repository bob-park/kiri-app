import { useCliInstall, useCliStatus, useCliUninstall } from '@/domain/cli/queries/cli';
import Button from '@/shared/components/settings/Button';
import SettingRow from '@/shared/components/settings/SettingRow';
import SettingSection from '@/shared/components/settings/SettingSection';
import useI18n from '@/shared/hooks/useI18n';

// Command syntax, not prose: the same in every language.
const USAGE = `kiri status            # running jobs
kiri list              # whole queue
kiri add <url> [--quality 1080p] [--format mp4-h264] [--subs ko,en]
kiri stop <id>
kiri remove <id>
kiri transcode <file> --format mp4-hevc [--quality 720p] [--output <dir>]
kiri list --json       # machine-readable`;

export default function CliTab() {
  // hooks
  const { t } = useI18n();

  // queries
  const { status } = useCliStatus();
  const { installCli } = useCliInstall();
  const { uninstallCli } = useCliUninstall();

  return (
    <>
      <SettingSection>
        <SettingRow
          label={t.settings.cli.status}
          desc={status?.installed ? t.settings.cli.installed(status.link) : t.settings.cli.notInstalled}
        >
          {status?.installed ? (
            <Button variant="outline" onClick={() => uninstallCli()}>
              {t.settings.cli.uninstall}
            </Button>
          ) : (
            <Button variant="primary" onClick={() => installCli()}>
              {t.settings.cli.install}
            </Button>
          )}
        </SettingRow>
      </SettingSection>
      {status?.socket_error && (
        <div role="alert" className="alert alert-warning mt-2 py-2 text-sm">
          {t.settings.cli.socketError(status.socket_error)}
        </div>
      )}
      <div className="text-fg-muted mt-3 mb-1 text-xs font-semibold">{t.settings.cli.usage}</div>
      <pre className="surface-card overflow-x-auto p-3 text-xs select-text">{USAGE}</pre>
    </>
  );
}
