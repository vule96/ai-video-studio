import clsx from 'clsx';
import { Clapperboard, Settings } from 'lucide-react';
import { useState } from 'react';

import { JobTray } from './features/jobs/JobTray';
import { ProjectScreen } from './features/project/ProjectScreen';
import { NewProjectScreen } from './features/projects/NewProjectScreen';
import { ProjectsScreen } from './features/projects/ProjectsScreen';
import { SettingsScreen } from './features/settings/SettingsScreen';
import { useBackendEvents } from './lib/queries';
import type { Navigate, Route } from './route';

export function App() {
  useBackendEvents();
  const [route, navigate] = useState<Route>({ name: 'projects' });

  return (
    <div className="grid h-full grid-cols-[216px_1fr]">
      <Sidebar route={route} navigate={navigate} />
      <main className="min-w-0 overflow-y-auto">
        <Screen route={route} navigate={navigate} />
      </main>
    </div>
  );
}

function Screen({ route, navigate }: { route: Route; navigate: Navigate }) {
  switch (route.name) {
    case 'projects':
      return <ProjectsScreen navigate={navigate} />;
    case 'new':
      return <NewProjectScreen navigate={navigate} />;
    case 'project':
      // key: switching projects resets local editor state.
      return <ProjectScreen key={route.id} id={route.id} navigate={navigate} />;
    case 'settings':
      return <SettingsScreen />;
  }
}

function Sidebar({ route, navigate }: { route: Route; navigate: Navigate }) {
  const items = [
    { label: 'Dự án', icon: Clapperboard, target: { name: 'projects' }, active: route.name !== 'settings' },
    { label: 'Cài đặt', icon: Settings, target: { name: 'settings' }, active: route.name === 'settings' },
  ] as const;

  return (
    <aside className="flex min-h-0 flex-col border-r border-line bg-rail">
      <div className="flex items-center gap-2.5 px-5 pt-6 pb-7">
        <img src="/icon.png" alt="" className="size-6" />
        <span className="text-[14px] font-semibold">AI Video Studio</span>
      </div>
      <nav className="flex flex-col gap-0.5 px-3" aria-label="Chính">
        {items.map(({ label, icon: Icon, target, active }) => (
          <button
            key={label}
            type="button"
            aria-current={active ? 'page' : undefined}
            onClick={() => {
              navigate(target);
            }}
            className={clsx(
              'flex items-center gap-2.5 rounded px-2.5 py-1.5 text-left',
              active ? 'bg-sheet text-ink' : 'text-dim hover:bg-sheet/60 hover:text-ink',
            )}
          >
            <Icon size={16} aria-hidden className={active ? 'text-ink' : 'text-faint'} />
            {label}
          </button>
        ))}
      </nav>
      <div className="mt-auto flex min-h-0 flex-col border-t border-line">
        <p className="px-5 pt-4 text-[12px] font-medium text-faint">Hàng đợi</p>
        <div className="min-h-0 overflow-y-auto">
          <JobTray navigate={navigate} />
        </div>
      </div>
    </aside>
  );
}
