import clsx from 'clsx';
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
    <div className="grid h-full grid-cols-[224px_1fr]">
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
    { label: 'Dự án', target: { name: 'projects' }, active: route.name !== 'settings' },
    { label: 'Cài đặt', target: { name: 'settings' }, active: route.name === 'settings' },
  ] as const;

  return (
    <aside className="flex min-h-0 flex-col border-r border-line bg-rail">
      <div className="flex items-center gap-2.5 px-4 pt-5 pb-6">
        <img src="/icon.png" alt="" className="size-7" />
        <span className="font-medium">AI Video Studio</span>
      </div>
      <nav className="flex flex-col gap-0.5 px-2" aria-label="Chính">
        {items.map((item) => (
          <button
            key={item.label}
            type="button"
            aria-current={item.active ? 'page' : undefined}
            onClick={() => {
              navigate(item.target);
            }}
            className={clsx(
              'rounded-md px-3 py-1.5 text-left',
              item.active ? 'bg-desk text-ink' : 'text-dim hover:bg-desk/60 hover:text-ink',
            )}
          >
            {item.label}
          </button>
        ))}
      </nav>
      <div className="mt-auto min-h-0 overflow-y-auto border-t border-line">
        <JobTray navigate={navigate} />
      </div>
    </aside>
  );
}
