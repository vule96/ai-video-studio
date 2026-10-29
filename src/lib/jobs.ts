import { create } from 'zustand';

import type { JobInfo } from './bindings/JobInfo';

interface JobsState {
  /** Insertion order is kept: newest last, like the backend list. */
  byId: ReadonlyMap<string, JobInfo>;
  upsert: (job: JobInfo) => void;
  replaceAll: (jobs: JobInfo[]) => void;
}

/** Live job list, fed by the `job` event (see `useBackendEvents`). */
export const useJobs = create<JobsState>((set) => ({
  byId: new Map(),
  upsert: (job) => {
    set((s) => {
      const next = new Map(s.byId);
      next.set(job.id, job);
      return { byId: next };
    });
  },
  replaceAll: (jobs) => {
    set({ byId: new Map(jobs.map((j) => [j.id, j])) });
  },
}));

export function isActive(job: JobInfo): boolean {
  return job.status === 'queued' || job.status === 'running';
}

/** What a project's queued/running jobs are working on, as keys like `shot:S01`, `character:toru`, `review:S01`, `clip:S01`, `storyboard`, `render`. */
export function busyTargets(jobs: Iterable<JobInfo>, projectId: string): Set<string> {
  const out = new Set<string>();
  for (const job of jobs) {
    if (job.projectId !== projectId || !isActive(job)) continue;
    const { spec } = job;
    switch (spec.type) {
      case 'storyboard':
        out.add('storyboard');
        break;
      case 'characterTakes':
        out.add(`character:${spec.characterId}`);
        break;
      case 'shotTakes':
        for (const id of spec.shotIds) out.add(`shot:${id}`);
        break;
      case 'render':
        out.add('render');
        break;
      case 'review':
        for (const id of spec.shotIds) out.add(`review:${id}`);
        break;
      case 'fix':
        for (const id of spec.shotIds) out.add(`shot:${id}`);
        break;
      case 'clips':
        for (const id of spec.shotIds) out.add(`clip:${id}`);
        break;
    }
  }
  return out;
}
