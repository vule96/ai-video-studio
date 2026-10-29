import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect } from 'react';

import type { JobSpec } from './bindings/JobSpec';
import type { ProjectPatch } from './bindings/ProjectPatch';
import { api, events } from './ipc';
import { useJobs } from './jobs';

export const keys = {
  root: ['projects-root'] as const,
  projects: ['projects'] as const,
  project: (id: string) => ['project', id] as const,
  settings: ['settings'] as const,
  health: ['health'] as const,
};

export function useProjectsRoot() {
  return useQuery({ queryKey: keys.root, queryFn: api.projectsRoot, staleTime: Infinity });
}

export function useProjects() {
  return useQuery({ queryKey: keys.projects, queryFn: api.projectsList });
}

export function useProject(id: string) {
  return useQuery({ queryKey: keys.project(id), queryFn: () => api.projectGet(id) });
}

export function useSettings() {
  return useQuery({ queryKey: keys.settings, queryFn: api.settingsGet, staleTime: Infinity });
}

export function useHealth() {
  return useQuery({ queryKey: keys.health, queryFn: api.healthCheck, staleTime: 30_000 });
}

export function usePatchProject(id: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (patch: ProjectPatch) => api.projectPatch(id, patch),
    onSuccess: (project) => {
      qc.setQueryData(keys.project(id), project);
      void qc.invalidateQueries({ queryKey: keys.projects });
    },
  });
}

export function useEnqueue() {
  const upsert = useJobs((s) => s.upsert);
  return useMutation({ mutationFn: (spec: JobSpec) => api.jobEnqueue(spec), onSuccess: upsert });
}

/** Mount once: mirrors backend jobs into the store and refetches projects the jobs change. */
export function useBackendEvents() {
  const qc = useQueryClient();
  const upsert = useJobs((s) => s.upsert);
  const replaceAll = useJobs((s) => s.replaceAll);

  useEffect(() => {
    let disposed = false;
    const unlisten = Promise.all([
      events.onJob(upsert),
      events.onProjectChanged((id) => {
        void qc.invalidateQueries({ queryKey: keys.project(id) });
        void qc.invalidateQueries({ queryKey: keys.projects });
      }),
    ]);
    // Listen first, then load: a job that changes in between is not missed.
    void unlisten
      .then(() => api.jobsList())
      .then((jobs) => {
        if (!disposed) replaceAll(jobs);
      });
    return () => {
      disposed = true;
      void unlisten.then((fns) => {
        for (const fn of fns) fn();
      });
    };
  }, [qc, upsert, replaceAll]);
}
