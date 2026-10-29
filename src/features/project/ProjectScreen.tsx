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

/** One line under each step: how far that step is, so the next thing to do is visible from any step. */
function stageStatus(p: Project, stage: Stage): string {
  switch (stage) {
    case 'script':
      return `${String(p.shots.length)} shot`;
    case 'characters': {
      const chosen = p.characters.filter((c) => c.selectedTake).length;
      return p.characters.length === 0
        ? 'Không có'
        : `${String(chosen)}/${String(p.characters.length)} đã chọn`;
    }
    case 'frames': {
      const drawn = p.shots.filter((s) => s.selectedTake).length;
      const flagged = p.shots.filter(
        (s) => s.takes.find((t) => t.id === s.selectedTake)?.review?.ok === false,
      ).length;
      return `${String(drawn)}/${String(p.shots.length)} đã vẽ${flagged > 0 ? `, ${String(flagged)} lỗi` : ''}`;
    }
    case 'render': {
      if (!p.video) return 'Chưa dựng';
      const s = Math.round(p.video.durationMs / 1000);
      return `Đã dựng ${String(Math.floor(s / 60))}:${String(s % 60).padStart(2, '0')}`;
    }
  }
}

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
      <header className="sticky top-0 z-10 border-b border-line bg-desk/95 backdrop-blur">
        <div className="mx-auto flex w-full max-w-[1360px] flex-col gap-4 px-8 pt-6">
          <div className="flex flex-col gap-1">
            <h1 className="truncate text-[21px] leading-tight font-semibold">{p.title}</h1>
            <p className="text-[13px] text-faint">
              {p.sourceKind === 'pdf' ? `Từ PDF: ${p.sourceLabel}` : 'Từ mô tả'},{' '}
              {p.format === 'landscape' ? 'ngang 16:9' : 'dọc 9:16'}
            </p>
          </div>
          {/* The stages are the actual order of work, so they are numbered. */}
          <nav aria-label="Các bước" className="-mb-px grid grid-cols-4">
            {STAGES.map((s, i) => {
              const current = stage === s.id;
              return (
                <button
                  key={s.id}
                  type="button"
                  aria-current={current ? 'step' : undefined}
                  onClick={() => {
                    setStage(s.id);
                  }}
                  className={clsx(
                    'flex items-center gap-3 border-b-2 py-3 pr-4 text-left',
                    current ? 'border-marker' : 'border-transparent hover:border-edge',
                  )}
                >
                  <span
                    className={clsx(
                      'flex size-6 shrink-0 items-center justify-center rounded-full text-[12px] font-semibold tabular-nums',
                      current ? 'bg-marker text-marker-ink' : 'bg-sheet text-dim',
                    )}
                  >
                    {i + 1}
                  </span>
                  <span className="flex min-w-0 flex-col leading-tight">
                    <span className={clsx('font-medium', current ? 'text-ink' : 'text-dim')}>{s.label}</span>
                    <span className="truncate text-[12px] text-faint">{stageStatus(p, s.id)}</span>
                  </span>
                </button>
              );
            })}
          </nav>
        </div>
      </header>

      <div className="mx-auto w-full max-w-[1360px] flex-1 px-8 pt-6 pb-12">
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
