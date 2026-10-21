import { useState } from 'react';

import type { Preset } from '@/domain/settings/apis/settings.dto';
import { useSettings, useSettingsPatch } from '@/domain/settings/queries/settings';
import Button from '@/shared/components/settings/Button';
import Segmented from '@/shared/components/settings/Segmented';
import SettingRow from '@/shared/components/settings/SettingRow';
import SettingSection from '@/shared/components/settings/SettingSection';
import Switch from '@/shared/components/settings/Switch';
import useI18n from '@/shared/hooks/useI18n';
import { useStore } from '@/shared/store/rootStore';
import { PRESETS } from '@/utils/sheet';

import { open } from '@tauri-apps/plugin-dialog';

const QUALITIES = ['best', '2160p', '1440p', '1080p', '720p', '480p', 'audio'];

const CONCURRENCY = [1, 2, 3, 4];

export default function DownloadTab() {
  // state
  // The subtitles field being edited; `null` shows the saved value.
  const [subsDraft, setSubsDraft] = useState<string | null>(null);

  // store
  const showError = useStore((s) => s.showError);

  // hooks
  const { t } = useI18n();

  // queries
  const { settings } = useSettings();
  const { patchSettings } = useSettingsPatch();

  const download = settings.download;

  // handle
  const handlePickDir = async () => {
    try {
      const dir = await open({ directory: true, defaultPath: download.dir });
      if (typeof dir === 'string') {
        patchSettings({ download: { dir } });
      }
    } catch (e) {
      showError({ code: 'UNKNOWN', message: String(e) });
    }
  };
  const handleCommitSubs = () => {
    if (subsDraft === null) {
      return;
    }
    setSubsDraft(null);
    patchSettings({
      download: {
        subtitles: subsDraft
          .split(',')
          .map((s) => s.trim())
          .filter(Boolean),
      },
    });
  };
  const qualityLabel = (q: string) => (q === 'best' ? t.quality.best : q === 'audio' ? t.quality.audio : q);

  return (
    <>
      <SettingSection title={t.settings.group.save}>
        <SettingRow label={t.settings.download.dir} desc={download.dir}>
          <Button onClick={() => void handlePickDir()}>{t.settings.download.change}</Button>
        </SettingRow>
      </SettingSection>
      <SettingSection title={t.settings.group.defaults}>
        <SettingRow label={t.settings.download.quality}>
          {(labelId) => (
            <select
              aria-labelledby={labelId}
              className="select select-sm w-40"
              value={download.quality}
              onChange={(e) => patchSettings({ download: { quality: e.target.value } })}
            >
              {QUALITIES.map((q) => (
                <option key={q} value={q}>
                  {qualityLabel(q)}
                </option>
              ))}
            </select>
          )}
        </SettingRow>
        <SettingRow label={t.settings.download.preset}>
          {(labelId) => (
            <select
              aria-labelledby={labelId}
              className="select select-sm w-40"
              value={download.preset}
              onChange={(e) => patchSettings({ download: { preset: e.target.value as Preset } })}
            >
              {PRESETS.map((p) => (
                <option key={p} value={p}>
                  {t.preset[p]}
                </option>
              ))}
            </select>
          )}
        </SettingRow>
        <SettingRow label={t.settings.download.subtitles} desc={t.settings.download.subtitlesHint}>
          {(labelId) => (
            <input
              aria-labelledby={labelId}
              className="input input-sm w-40"
              value={subsDraft ?? download.subtitles.join(',')}
              onChange={(e) => setSubsDraft(e.target.value)}
              onBlur={handleCommitSubs}
              onKeyDown={(e) => e.key === 'Enter' && handleCommitSubs()}
            />
          )}
        </SettingRow>
        <SettingRow label={t.settings.download.skipSheet}>
          {(labelId) => (
            <Switch
              labelledBy={labelId}
              checked={download.skip_sheet}
              onChange={(skip_sheet) => patchSettings({ download: { skip_sheet } })}
            />
          )}
        </SettingRow>
      </SettingSection>
      <SettingSection title={t.settings.group.performance}>
        <SettingRow label={t.settings.download.maxConcurrent}>
          {(labelId) => (
            <Segmented
              labelledBy={labelId}
              itemClassName="w-9"
              value={download.max_concurrent}
              options={CONCURRENCY.map((n) => ({ value: n, label: String(n) }))}
              onChange={(max_concurrent) => patchSettings({ download: { max_concurrent } })}
            />
          )}
        </SettingRow>
        <SettingRow label={t.settings.download.hwAccel} desc={t.settings.download.hwAccelDesc}>
          {(labelId) => (
            <Switch
              labelledBy={labelId}
              checked={download.hw_accel}
              onChange={(hw_accel) => patchSettings({ download: { hw_accel } })}
            />
          )}
        </SettingRow>
      </SettingSection>
    </>
  );
}
