import { describe, expect, it } from 'vitest';

import type { JobInfo } from './bindings/JobInfo';
import type { JobSpec } from './bindings/JobSpec';
import type { JobStatus } from './bindings/JobStatus';
import { busyTargets } from './jobs';

function job(projectId: string, status: JobStatus, spec: JobSpec): JobInfo {
  return {
    id: Math.random().toString(36),
    projectId,
    spec,
    label: '',
    status,
    done: 0,
    total: 0,
    message: '',
    error: null,
  };
}

describe('busyTargets', () => {
  it('collects active work for one project only', () => {
    const jobs = [
      job('p1', 'running', { type: 'shotTakes', projectId: 'p1', shotIds: ['S01', 'S02'] }),
      job('p1', 'queued', { type: 'characterTakes', projectId: 'p1', characterId: 'toru', count: 3 }),
      job('p1', 'done', { type: 'storyboard', projectId: 'p1', shotCount: 4 }),
      job('p1', 'queued', { type: 'render', projectId: 'p1', voice: 'Thiện Minh', burnSubtitles: false }),
      job('p1', 'queued', { type: 'review', projectId: 'p1', shotIds: ['S02'] }),
      job('p2', 'running', { type: 'storyboard', projectId: 'p2', shotCount: 4 }),
    ];
    expect([...busyTargets(jobs, 'p1')].sort()).toEqual([
      'character:toru',
      'render',
      'review:S02',
      'shot:S01',
      'shot:S02',
    ]);
  });
});
