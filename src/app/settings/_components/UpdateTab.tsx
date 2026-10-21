import { useEffect, useState } from 'react';

import type { Job } from '@/domain/queue/apis/queue.dto';
import { useSettings, useSettingsPatch } from '@/domain/settings/queries/settings';
import type { ToolsStatus } from '@/domain/tools/apis/tools.dto';
import { useToolsUpdate } from '@/domain/tools/queries/tools';
import { useUpdateCheck, useUpdateDownloadRetry, useUpdateInstall } from '@/domain/updater/queries/updater';
import Button from '@/shared/components/settings/Button';
import SettingRow from '@/shared/components/settings/SettingRow';
import SettingSection from '@/shared/components/settings/SettingSection';
import Switch from '@/shared/components/settings/Switch';
import useI18n from '@/shared/hooks/useI18n';
import { isIdle } from '@/utils/queue';
import { type UpdateView, updatePct } from '@/utils/update';

import { getVersion } from '@tauri-apps/api/app';

interface UpdateTabProps {
  jobs: Job[];
  tools: ToolsStatus | null;
  update: UpdateView;
}

/** The data comes from SettingsApp, whose listeners live as long as the window (this tab unmounts). */
export default function UpdateTab({ jobs, tools, update }: Readonly<UpdateTabProps>) {
  // state
  const [version, setVersion] = useState('');

  // hooks
  const { t } = useI18n();

  // queries
  const { settings } = useSettings();
  const { patchSettings } = useSettingsPatch();
  const { updateTools, isLoading: toolsUpdating } = useToolsUpdate();
  const { checkUpdate, isLoading: checking } = useUpdateCheck();
  const { installUpdate } = useUpdateInstall();
  const { retryUpdateDownload } = useUpdateDownloadRetry();

  // useEffect
  useEffect(() => {
    void getVersion().then(setVersion);
  }, []);

  const { info, phase, progress, checked } = update;
  const idle = isIdle(jobs);
  const next = info?.version ?? '';
  const percent = updatePct(progress);
  const appDesc = (() => {
    if (!info || phase === 'idle') {
      return checked ? t.settings.update.upToDate : undefined;
    }
    switch (phase) {
      case 'downloading':
        return `${t.update.downloadingBg(next)}${percent !== null ? ` ${percent}%` : ''}`;
      case 'ready':
        return t.update.readyTitle(next);
      case 'failed':
        return t.update.downloadFailed;
      case 'scheduled':
        return t.update.scheduled;
      case 'installing':
        return t.update.installing;
    }
  })();
  const toolsBusy = toolsUpdating || !!tools?.installing;
  const lastCheck = tools?.last_check ? new Date(tools.last_check).toLocaleString() : t.settings.update.never;

  return (
    <>
      <SettingSection title="kiri">
        <SettingRow label={t.settings.update.appVersion(version)} desc={appDesc}>
          <div className="flex gap-1.5">
            {phase === 'ready' && (
              <Button variant="primary" onClick={() => installUpdate(!idle)}>
                {idle ? t.update.restartInstall : t.update.afterQueue}
              </Button>
            )}
            {phase === 'failed' && <Button onClick={() => retryUpdateDownload()}>{t.update.retry}</Button>}
            <Button disabled={checking} onClick={() => checkUpdate()}>
              {checking ? t.settings.update.checking : t.settings.update.check}
            </Button>
          </div>
        </SettingRow>
        <SettingRow label={t.settings.update.autoCheck}>
          {(labelId) => (
            <Switch
              labelledBy={labelId}
              checked={settings.update.auto_check}
              onChange={(auto_check) => patchSettings({ update: { auto_check } })}
            />
          )}
        </SettingRow>
      </SettingSection>
      <SettingSection title="yt-dlp">
        <SettingRow
          label={
            tools?.ytdlp_version ? t.settings.update.ytdlpVersion(tools.ytdlp_version) : t.settings.update.ytdlpMissing
          }
          desc={t.settings.update.lastCheck(lastCheck)}
        >
          <Button aria-label={t.settings.update.ytdlpUpdate} disabled={toolsBusy} onClick={() => updateTools()}>
            {toolsBusy ? <span className="loading loading-spinner loading-xs" /> : t.settings.update.ytdlpUpdate}
          </Button>
        </SettingRow>
      </SettingSection>
      {tools?.error && !tools.installing && (
        <div role="alert" className="alert alert-error mt-2 py-2 text-sm">
          {tools.error}
        </div>
      )}
    </>
  );
}
