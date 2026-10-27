import type { ToolName, ToolsStatus } from '@/domain/tools/apis/tools.dto';
import { useToolUpdateCheck, useToolsUpdate } from '@/domain/tools/queries/tools';
import Button from '@/shared/components/settings/Button';
import SettingRow from '@/shared/components/settings/SettingRow';
import SettingSection from '@/shared/components/settings/SettingSection';
import useI18n from '@/shared/hooks/useI18n';
import useOnline from '@/shared/hooks/useOnline';
import { TOOL_ORDER, pendingUpdates, toolInfo } from '@/utils/tools';

const LABEL_KEY = { 'yt-dlp': 'ytdlp', ffmpeg: 'ffmpeg', deno: 'deno' } as const satisfies Record<ToolName, string>;

interface ToolsSectionProps {
  tools: ToolsStatus | null;
}

/** Installed and latest versions of yt-dlp, FFmpeg and Deno, with per-tool and "update all" buttons. */
export default function ToolsSection({ tools }: Readonly<ToolsSectionProps>) {
  // hooks
  const { t } = useI18n();
  const online = useOnline();

  // queries
  const { updateTools, isLoading } = useToolsUpdate();
  useToolUpdateCheck(tools?.installing);

  const pending = pendingUpdates(tools);
  const disabled = isLoading || !!tools?.installing || !online;
  const lastCheck = tools?.last_check ? new Date(tools.last_check).toLocaleString() : t.settings.update.never;

  return (
    <SettingSection title={t.settings.update.components}>
      <SettingRow
        label={pending.length > 0 ? t.settings.update.updatesAvailable(pending.length) : t.settings.update.allUpToDate}
        desc={t.settings.update.lastCheck(lastCheck)}
      >
        {pending.length > 0 && (
          <Button variant="primary" disabled={disabled} onClick={() => updateTools(pending)}>
            {t.settings.update.updateAll}
          </Button>
        )}
      </SettingRow>
      {TOOL_ORDER.map((name) => {
        const { current, latest } = tools ? toolInfo(tools, name) : { current: null, latest: null };
        const hasUpdate = pending.includes(name);
        const desc = (() => {
          if (hasUpdate && latest) {
            return current ? t.settings.update.versionArrow(current, latest) : t.settings.update.notInstalled;
          }

          return current ? t.settings.update.versionLatest(current) : t.settings.update.notInstalled;
        })();

        return (
          <SettingRow key={name} label={t.settings.update.toolLabel[LABEL_KEY[name]]} desc={desc}>
            {(labelId) =>
              hasUpdate ? (
                <Button aria-describedby={labelId} disabled={disabled} onClick={() => updateTools([name])}>
                  {tools?.installing === name ? (
                    <span className="loading loading-spinner loading-xs" aria-label={t.settings.update.update} />
                  ) : (
                    t.settings.update.update
                  )}
                </Button>
              ) : (
                current && (
                  <span className="text-success text-sm" aria-hidden>
                    ✓
                  </span>
                )
              )
            }
          </SettingRow>
        );
      })}
    </SettingSection>
  );
}
