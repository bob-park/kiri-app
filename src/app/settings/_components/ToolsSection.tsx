import type { ToolName, ToolsStatus } from '@/domain/tools/apis/tools.dto';
import { useToolUpdateCheck, useToolsUpdate } from '@/domain/tools/queries/tools';
import Button from '@/shared/components/settings/Button';
import SettingRow from '@/shared/components/settings/SettingRow';
import SettingSection from '@/shared/components/settings/SettingSection';
import useI18n from '@/shared/hooks/useI18n';
import useOnline from '@/shared/hooks/useOnline';
import { TOOL_ORDER, pendingUpdates, toolInfo, toolState, toolsSummary } from '@/utils/tools';

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
  const { checking } = useToolUpdateCheck(tools?.installing);

  const pending = pendingUpdates(tools);
  const disabled = isLoading || checking || !!tools?.installing || !online;
  const lastCheck = tools?.last_check ? new Date(tools.last_check).toLocaleString() : t.settings.update.never;
  const summary = {
    updates: t.settings.update.updatesAvailable(pending.length),
    upToDate: t.settings.update.allUpToDate,
    unknown: t.settings.update.componentVersions,
  }[toolsSummary(tools)];

  return (
    <SettingSection title={t.settings.update.components}>
      <SettingRow label={summary} desc={t.settings.update.lastCheck(lastCheck)}>
        {pending.length > 0 && (
          <Button variant="primary" disabled={disabled} onClick={() => updateTools(pending)}>
            {t.settings.update.updateAll}
          </Button>
        )}
      </SettingRow>
      {TOOL_ORDER.map((name) => {
        const info = tools ? toolInfo(tools, name) : { current: null, latest: null };
        const state = toolState(info);
        const desc = {
          update:
            info.current && info.latest
              ? t.settings.update.versionArrow(info.current, info.latest)
              : t.settings.update.notInstalled,
          latest: t.settings.update.versionLatest(info.current ?? ''),
          unknown: info.current ?? '',
          missing: t.settings.update.notInstalled,
        }[state];
        const spinner = (
          <span role="status" aria-label={t.settings.update.updating} className="loading loading-spinner loading-xs" />
        );

        return (
          <SettingRow key={name} label={t.settings.update.toolLabel[LABEL_KEY[name]]} desc={desc}>
            {(labelId) => {
              if (state === 'update') {
                return (
                  <Button aria-describedby={labelId} disabled={disabled} onClick={() => updateTools([name])}>
                    {tools?.installing === name ? spinner : t.settings.update.update}
                  </Button>
                );
              }
              if (tools?.installing === name) {
                return spinner;
              }

              return (
                state === 'latest' && (
                  <span className="text-success text-sm" aria-hidden>
                    ✓
                  </span>
                )
              );
            }}
          </SettingRow>
        );
      })}
    </SettingSection>
  );
}
