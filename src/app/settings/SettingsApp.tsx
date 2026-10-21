import { useState } from 'react';

import CliTab from '@/app/settings/_components/CliTab';
import DownloadTab from '@/app/settings/_components/DownloadTab';
import GeneralTab from '@/app/settings/_components/GeneralTab';
import UpdateTab from '@/app/settings/_components/UpdateTab';
import { useJobs } from '@/domain/queue/queries/queue';
import { useToolsStatus } from '@/domain/tools/queries/tools';
import { useUpdateStatus } from '@/domain/updater/queries/updater';
import Icon from '@/shared/components/Icon';
import useI18n from '@/shared/hooks/useI18n';

import cx from 'classnames';

const TABS = [
  { key: 'general', icon: 'settings', Body: GeneralTab },
  { key: 'download', icon: 'download', Body: DownloadTab },
  { key: 'cli', icon: 'terminal', Body: CliTab },
  { key: 'update', icon: 'refresh', Body: null },
] as const;

const TAB_WIDTH = 72;

type TabKey = (typeof TABS)[number]['key'];

/** The `settings` window. */
export default function SettingsApp() {
  // state
  const [tab, setTab] = useState<TabKey>('general');

  // hooks
  const { t } = useI18n();

  // queries
  // Read here, not in UpdateTab: their event listeners must live as long as the window,
  // or an update://status (or queue/tools) event sent while another tab is open would be lost.
  const { jobs } = useJobs();
  const { status: tools } = useToolsStatus();
  const update = useUpdateStatus();

  const index = TABS.findIndex((x) => x.key === tab);
  const Body = TABS[index].Body;

  return (
    <div className="relative flex h-full flex-col">
      <div className="aurora" aria-hidden="true">
        <span className="aurora-blob" />
        <span className="aurora-blob" />
        <span className="aurora-blob" />
      </div>
      <div className="relative flex justify-center px-2 pt-2 pb-2.5">
        <nav role="tablist" className="glass shadow-glass relative flex gap-1 rounded-2xl p-1">
          {/* Slides under the selected tab: tabs are TAB_WIDTH wide with a 4px gap. */}
          <span
            aria-hidden="true"
            className="bg-secondary ease-spring absolute inset-y-1 left-1 rounded-xl transition-transform duration-450 motion-reduce:transition-none"
            style={{ width: TAB_WIDTH, transform: `translateX(${index * (TAB_WIDTH + 4)}px)` }}
          />
          {TABS.map((x) => (
            <button
              key={x.key}
              role="tab"
              aria-selected={tab === x.key}
              className={cx(
                'focus-visible:outline-primary relative flex flex-col items-center gap-0.5 rounded-xl px-2 py-1.5 text-[11px] transition-colors duration-200 focus-visible:outline-2 focus-visible:outline-offset-2 motion-reduce:transition-none',
                {
                  'text-secondary-content font-semibold': tab === x.key,
                  'text-muted hover:text-base-content': tab !== x.key,
                },
              )}
              style={{ width: TAB_WIDTH }}
              onClick={() => setTab(x.key)}
            >
              <Icon name={x.icon} className="h-[18px] w-[18px]" />
              {t.settings.tab[x.key]}
            </button>
          ))}
        </nav>
      </div>
      <div className="relative flex-1 overflow-y-auto px-5 pt-1 pb-4">
        {Body ? <Body /> : <UpdateTab jobs={jobs} tools={tools} update={update} />}
      </div>
    </div>
  );
}
