import clsx from 'clsx';
import { useMemo, useState } from 'react';

import type { Project } from '@/lib/bindings/Project';
import { errorMessage } from '@/lib/ipc';
import { busyTargets, isActive, useJobs } from '@/lib/jobs';
import { useProject, useProjectsRoot } from '@/lib/queries';
import type { Navigate, Stage } from '@/route';
import { ErrorText, Progress } from '@/ui';

import { CharactersStage } from './CharactersStage';
import { FramesStage } from './FramesStage';
import { RenderStage } from './RenderStage';
import { ScriptStage } from './ScriptStage';

/** Everything a stage needs about the open project. */
export interface StageProps {
  project: Project;
  root: string;
  /** Keys like `shot:S01`, see `busyTargets`. */
  busy: Set<string>;
}

const STAGES: readonly { id: Stage; label: string }[] = [
  { id: 'script', label: 'Kịch bản' },
  { id: 'characters', label: 'Nhân vật' },
  { id: 'frames', label: 'Khung hình' },
  { id: 'render', label: 'Dựng phim' },
];

export function ProjectScreen({ id, navigate }: { id: string; navigate: Navigate }) {
  const project = useProject(id);
  const root = useProjectsRoot();
  const byId = useJobs((s) => s.byId);
  const busy = useMemo(() => busyTargets(byId.values(), id), [byId, id]);
  const [stage, setStage] = useState<Stage>('script');

  if (project.error) {
    return (
      <div className="px-8 py-8">
        <ErrorText>{errorMessage(project.error)}</ErrorText>
        <button
          type="button"
          className="mt-3 text-dim underline"
          onClick={() => {
            navigate({ name: 'projects' });
          }}
        >
          Về danh sách dự án
        </button>
      </div>
    );
  }
  if (!project.data || !root.data) return null;

  const p = project.data;
  const props: StageProps = { project: p, root: root.data, busy };

  return (
    <div className="flex min-h-full flex-col">
      <header className="sticky top-0 z-10 flex flex-col gap-3 border-b border-line bg-desk/95 px-8 pt-6 backdrop-blur">
        <div>
          <h1 className="text-[22px] font-medium">{p.title}</h1>
          <p className="text-[12px] text-faint">
            {p.sourceKind === 'pdf' ? `Từ PDF: ${p.sourceLabel}` : 'Từ mô tả'},{' '}
            {p.format === 'landscape' ? 'ngang 16:9' : 'dọc 9:16'}
          </p>
        </div>
        {/* The stages are the actual order of work, so they are numbered. */}
        <nav aria-label="Các bước" className="-mb-px flex gap-6">
          {STAGES.map((s, i) => (
            <button
              key={s.id}
              type="button"
              aria-current={stage === s.id ? 'step' : undefined}
              onClick={() => {
                setStage(s.id);
              }}
              className={clsx(
                'flex items-center gap-2 border-b-2 pb-2.5',
                stage === s.id ? 'border-marker text-ink' : 'border-transparent text-dim hover:text-ink',
              )}
            >
              <span className="text-[12px] text-faint tabular-nums">{i + 1}</span>
              {s.label}
            </button>
          ))}
        </nav>
      </header>

      <div className="flex-1 px-8 py-6">
        {busy.has('storyboard') ? (
          <StoryboardPending projectId={p.id} />
        ) : stage === 'script' ? (
          <ScriptStage {...props} />
        ) : stage === 'characters' ? (
          <CharactersStage {...props} />
        ) : stage === 'frames' ? (
          <FramesStage {...props} />
        ) : (
          <RenderStage {...props} />
        )}
      </div>
    </div>
  );
}

/** Shown while the LLM writes (or rewrites) the storyboard; editing now would be overwritten. */
function StoryboardPending({ projectId }: { projectId: string }) {
  const job = useJobs((s) =>
    [...s.byId.values()].find(
      (j) => j.projectId === projectId && j.spec.type === 'storyboard' && isActive(j),
    ),
  );
  return (
    <div className="flex max-w-md flex-col gap-3 py-10">
      <p className="text-[16px]">
        {job?.status === 'queued' ? 'Đang chờ đến lượt…' : 'LLM đang viết storyboard…'}
      </p>
      <Progress done={job?.done ?? 0} total={job?.total ?? 0} />
      <p className="text-[13px] text-dim">
        {job?.message}. Thường mất 1 đến 3 phút; sách dài lâu hơn vì phải tóm tắt từng phần.
      </p>
    </div>
  );
}
