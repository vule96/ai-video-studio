import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import type { Health } from './bindings/Health';
import type { JobInfo } from './bindings/JobInfo';
import type { JobSpec } from './bindings/JobSpec';
import type { NewProject } from './bindings/NewProject';
import type { Project } from './bindings/Project';
import type { ProjectPatch } from './bindings/ProjectPatch';
import type { ProjectSummary } from './bindings/ProjectSummary';
import type { Settings } from './bindings/Settings';

/** Shape of every error a command rejects with (see `src-tauri/src/error.rs`). */
interface AppError {
  kind: string;
  message: string;
}

function isAppError(e: unknown): e is AppError {
  return typeof e === 'object' && e !== null && 'message' in e && typeof e.message === 'string';
}

export function errorMessage(e: unknown): string {
  if (isAppError(e)) return e.message;
  return e instanceof Error ? e.message : String(e);
}

/** Typed wrappers around the Rust commands registered in `src-tauri/src/lib.rs`. */
export const api = {
  projectsRoot: () => invoke<string>('projects_root'),
  projectsList: () => invoke<ProjectSummary[]>('projects_list'),
  projectGet: (id: string) => invoke<Project>('project_get', { id }),
  projectCreate: (input: NewProject) => invoke<Project>('project_create', { input }),
  projectPatch: (id: string, patch: ProjectPatch) => invoke<Project>('project_patch', { id, patch }),
  projectDelete: (id: string) => invoke<null>('project_delete', { id }),
  /** Sends the file as the raw request body; a JSON number array would be ~4x the size. */
  projectMusicSet: async (id: string, file: File) =>
    invoke<Project>('project_music_set', new Uint8Array(await file.arrayBuffer()), {
      headers: { 'project-id': id, 'file-name': encodeURIComponent(file.name) },
    }),
  projectMusicRemove: (id: string) => invoke<Project>('project_music_remove', { id }),
  projectVideoDelete: (id: string) => invoke<Project>('project_video_delete', { id }),
  /** Writes `script.json` in the project folder; returns its full path. */
  projectScriptExport: (id: string) => invoke<string>('project_script_export', { id }),
  /** Applies `script.json` from the project folder; returns the project and how many shots changed. */
  projectScriptImport: (id: string) => invoke<[Project, number]>('project_script_import', { id }),
  jobEnqueue: (spec: JobSpec) => invoke<JobInfo>('job_enqueue', { spec }),
  jobsList: () => invoke<JobInfo[]>('jobs_list'),
  jobCancel: (id: string) => invoke<null>('job_cancel', { id }),
  settingsGet: () => invoke<Settings>('settings_get'),
  settingsSave: (settings: Settings) => invoke<Settings>('settings_save', { settings }),
  healthCheck: () => invoke<Health>('health_check'),
};

/** Backend events; names match the constants in `src-tauri/src/lib.rs`. */
export const events = {
  onJob: (cb: (job: JobInfo) => void): Promise<UnlistenFn> =>
    listen<JobInfo>('job', (e) => {
      cb(e.payload);
    }),
  onProjectChanged: (cb: (projectId: string) => void): Promise<UnlistenFn> =>
    listen<string>('project-changed', (e) => {
      cb(e.payload);
    }),
};

/** URL for an image stored in a project folder (served by Tauri's asset protocol). */
export function projectFileUrl(root: string, projectId: string, file: string): string {
  const sep = root.includes('\\') ? '\\' : '/';
  return convertFileSrc([root, projectId, ...file.split('/')].join(sep));
}
