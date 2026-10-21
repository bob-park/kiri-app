import { useState } from 'react';

import CliTab from '@/app/settings/_components/CliTab';
import DownloadTab from '@/app/settings/_components/DownloadTab';
import GeneralTab from '@/app/settings/_components/GeneralTab';
import UpdateTab from '@/app/settings/_components/UpdateTab';
import Icon from '@/shared/components/Icon';
import useI18n from '@/shared/hooks/useI18n';

import cx from 'classnames';

const TABS = [
  { key: 'general', icon: 'settings', Body: GeneralTab },
  { key: 'download', icon: 'download', Body: DownloadTab },
  { key: 'cli', icon: 'terminal', Body: CliTab },
  { key: 'update', icon: 'refresh', Body: UpdateTab },
] as const;

type TabKey = (typeof TABS)[number]['key'];

/** The `settings` window. */
export default function SettingsApp() {
  // state
  const [tab, setTab] = useState<TabKey>('general');

  // hooks
  const { t } = useI18n();

  const Body = TABS.find((x) => x.key === tab)?.Body ?? GeneralTab;

  return (
    <div className="bg-base-200 flex h-full flex-col">
      <nav role="tablist" className="flex justify-center gap-1 px-2 pt-2 pb-2.5">
        {TABS.map((x) => (
          <button
            key={x.key}
            role="tab"
            aria-selected={tab === x.key}
            className={cx(
              'flex w-[72px] flex-col items-center gap-0.5 rounded-xl px-2 py-1.5 text-[11px] transition-colors',
              {
                'bg-secondary text-secondary-content font-semibold': tab === x.key,
                'text-fg-muted hover:bg-base-300/60': tab !== x.key,
              },
            )}
            onClick={() => setTab(x.key)}
          >
            <Icon name={x.icon} className="h-[18px] w-[18px]" />
            {t.settings.tab[x.key]}
          </button>
        ))}
      </nav>
      <div className="flex-1 overflow-y-auto px-5 pt-1 pb-4">
        <Body />
      </div>
    </div>
  );
}
