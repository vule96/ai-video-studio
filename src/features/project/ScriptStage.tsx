import { ArrowDown, ArrowUp, Plus } from 'lucide-react';
import { useState } from 'react';

import type { Shot } from '@/lib/bindings/Shot';
import { useDraft } from '@/lib/draft';
import { errorMessage } from '@/lib/ipc';
import { useEnqueue, usePatchProject } from '@/lib/queries';
import { Button, ErrorText, Field, Input, NumberInput, Panel, Segmented, TextArea, Toolbar } from '@/ui';

import type { StageProps } from './ProjectScreen';
import { ShotFields, useShotDraft } from './ShotFields';

/** Left: what applies to the whole film. Right: the shot list, in film order. */
export function ScriptStage({ project, busy }: StageProps) {
  const patch = usePatchProject(project.id);
  const last = project.shots.at(-1)?.id ?? null;
  return (
    <div className="grid grid-cols-[360px_minmax(0,1fr)] items-start gap-8">
      <MetaEditor {...{ project }} />
      <section className="flex min-w-0 flex-col gap-4" aria-label="Danh sách shot">
        <Toolbar actions={<RewriteButton projectId={project.id} shotCount={project.shots.length || 6} />}>
          <span className="text-[15px] font-semibold text-ink">{project.shots.length} shot</span>
          <span className="ml-2">theo thứ tự trong phim</span>
        </Toolbar>
        {project.shots.length === 0 && (
          <p className="text-dim">
            Chưa có shot nào. Bấm “Viết lại storyboard” để LLM viết từ nội dung nguồn.
          </p>
        )}
        <ol className="flex flex-col gap-3">
          {project.shots.map((shot, i) => (
            <li key={shot.id} className="rounded-md bg-sheet p-5">
              <ShotCard
                projectId={project.id}
                shot={shot}
                project={project}
                position={{ first: i === 0, last: i === project.shots.length - 1 }}
                busy={busy.has(`shot:${shot.id}`) || busy.has('render')}
              />
            </li>
          ))}
        </ol>
        <div className="flex items-center gap-3">
          <Button
            disabled={patch.isPending}
            onClick={() => {
              patch.mutate({ type: 'addShot', after: last });
            }}
          >
            <Plus size={14} aria-hidden /> Thêm shot ở cuối
          </Button>
          {patch.error && <ErrorText>{errorMessage(patch.error)}</ErrorText>}
        </div>
      </section>
    </div>
  );
}

function MetaEditor({ project }: Pick<StageProps, 'project'>) {
  const patch = usePatchProject(project.id);
  const { draft, dirty, set, reset } = useDraft({
    title: project.title,
    style: project.style,
    format: project.format,
  });

  return (
    <Panel title="Toàn bộ phim" hint="Áp dụng cho mọi khung hình." className="sticky top-[176px]">
      <Field label="Tên dự án">
        {(id) => (
          <Input
            id={id}
            value={draft.title}
            onChange={(e) => {
              set({ title: e.target.value });
            }}
          />
        )}
      </Field>
      <Field
        label="Phong cách hình (tiếng Anh)"
        hint="Một câu dùng chung cho mọi ảnh: chất liệu, bảng màu, thời đại, không khí. Đây là thứ giữ cả video đồng nhất."
      >
        {(id) => (
          <TextArea
            id={id}
            rows={6}
            className="font-mono text-[12.5px]"
            value={draft.style}
            onChange={(e) => {
              set({ style: e.target.value });
            }}
          />
        )}
      </Field>
      <div className="flex flex-col gap-1">
        <span className="text-[12.5px] font-medium text-dim">Khung hình</span>
        <Segmented
          label="Khung hình"
          value={draft.format}
          onChange={(format) => {
            set({ format });
          }}
          options={[
            { value: 'landscape', label: 'Ngang 16:9' },
            { value: 'portrait', label: 'Dọc 9:16' },
          ]}
        />
      </div>
      {patch.error && <ErrorText>{errorMessage(patch.error)}</ErrorText>}
      <div>
        <Button
          variant="primary"
          disabled={!dirty || patch.isPending}
          onClick={() => {
            patch.mutate({ type: 'meta', ...draft }, { onSuccess: reset });
          }}
        >
          Lưu
        </Button>
      </div>
    </Panel>
  );
}

interface ShotCardProps extends Pick<StageProps, 'project'> {
  projectId: string;
  shot: Shot;
  position: { first: boolean; last: boolean };
  /** A job is drawing or rendering this shot; moving or deleting it now would lose that work. */
  busy: boolean;
}

function ShotCard({ projectId, shot, project, position, busy }: ShotCardProps) {
  const { draft, set, dirty, save, saving, error } = useShotDraft(projectId, shot);
  const patch = usePatchProject(projectId);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const iconButton =
    'rounded p-1 text-faint hover:bg-sheet hover:text-ink disabled:opacity-30 disabled:hover:bg-transparent';
  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center gap-1">
        <p className="mr-auto text-[13px] text-faint tabular-nums">{shot.id}</p>
        <button
          type="button"
          aria-label={`Đưa ${shot.id} lên`}
          title="Đưa lên"
          disabled={position.first || patch.isPending}
          className={iconButton}
          onClick={() => {
            patch.mutate({ type: 'moveShot', id: shot.id, by: -1 });
          }}
        >
          <ArrowUp size={15} />
        </button>
        <button
          type="button"
          aria-label={`Đưa ${shot.id} xuống`}
          title="Đưa xuống"
          disabled={position.last || patch.isPending}
          className={iconButton}
          onClick={() => {
            patch.mutate({ type: 'moveShot', id: shot.id, by: 1 });
          }}
        >
          <ArrowDown size={15} />
        </button>
        <button
          type="button"
          aria-label={`Thêm shot sau ${shot.id}`}
          title="Thêm shot ngay sau"
          disabled={patch.isPending}
          className={iconButton}
          onClick={() => {
            patch.mutate({ type: 'addShot', after: shot.id });
          }}
        >
          <Plus size={15} />
        </button>
        {confirmDelete ? (
          <span className="ml-2 flex items-center gap-1.5 text-[13px]">
            <span className="text-dim">Xoá {shot.id}?</span>
            <Button
              variant="danger"
              disabled={patch.isPending}
              onClick={() => {
                patch.mutate({ type: 'deleteShot', id: shot.id });
              }}
            >
              Xoá
            </Button>
            <Button
              onClick={() => {
                setConfirmDelete(false);
              }}
            >
              Thôi
            </Button>
          </span>
        ) : (
          <button
            type="button"
            disabled={busy}
            title={busy ? 'Shot đang được vẽ hoặc dựng' : undefined}
            className="ml-2 rounded px-1.5 py-0.5 text-[13px] text-faint hover:text-alert disabled:opacity-30"
            onClick={() => {
              setConfirmDelete(true);
            }}
          >
            Xoá
          </button>
        )}
      </div>
      <ShotFields project={project} draft={draft} onChange={set} compact />
      {(error ?? patch.error) && <ErrorText>{errorMessage(error ?? patch.error)}</ErrorText>}
      <div>
        <Button variant="primary" disabled={!dirty || saving} onClick={() => void save()}>
          Lưu shot
        </Button>
      </div>
    </div>
  );
}

function RewriteButton({ projectId, shotCount }: { projectId: string; shotCount: number }) {
  const enqueue = useEnqueue();
  // null: follow the current shot count until the user types a number.
  const [typed, setCount] = useState<number | null>(null);
  const count = typed ?? shotCount;
  // Rewriting replaces characters and shots (images stay on disk but are unlinked), so it asks first.
  const [confirming, setConfirming] = useState(false);
  return (
    <div className="flex flex-col items-end gap-1.5">
      <div className="flex items-center gap-2">
        <NumberInput
          min={1}
          max={40}
          aria-label="Số shot"
          className="w-16"
          value={count}
          onValueChange={setCount}
        />
        {confirming ? (
          <>
            <span className="text-[13px] text-dim">Thay toàn bộ nhân vật và shot?</span>
            <Button
              variant="danger"
              disabled={enqueue.isPending}
              onClick={() => {
                enqueue.mutate({ type: 'storyboard', projectId, shotCount: count });
                setConfirming(false);
              }}
            >
              Viết lại
            </Button>
            <Button
              onClick={() => {
                setConfirming(false);
              }}
            >
              Thôi
            </Button>
          </>
        ) : (
          <Button
            onClick={() => {
              setConfirming(true);
            }}
          >
            Viết lại storyboard
          </Button>
        )}
      </div>
      {enqueue.error && <ErrorText>{errorMessage(enqueue.error)}</ErrorText>}
    </div>
  );
}
