/** The app has four screens; a union beats a router dependency for that. */
export type Route =
  { name: 'projects' } | { name: 'new' } | { name: 'project'; id: string } | { name: 'settings' };

export type Stage = 'script' | 'characters' | 'frames' | 'render';

export type Navigate = (route: Route) => void;
