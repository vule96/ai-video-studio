import clsx from 'clsx';
import { Check, TriangleAlert, X } from 'lucide-react';
import { useState } from 'react';

import type { Shot } from '@/lib/bindings/Shot';
import { errorMessage, projectFileUrl } from '@/lib/ipc';
import { useEnqueue, usePatchProject } from '@/lib/queries';
import { Button, ErrorText } from '@/ui';

import type { StageProps } from './ProjectScreen';
import { ShotFields, useShotDraft } from './ShotFields';
import { TakeStrip } from './TakeStrip';

function selectedFile(shot: Shot): string | undefined {
  return shot.takes.find((t) => t.id === shot.selectedTake)?.file;
}

/** A review counts only for the frame it looked at; picking or drawing another take makes it stale. */
export function currentReview(shot: Shot) {
  return shot.review?.takeId === shot.selectedTake ? shot.review : null;
}

/** The storyboard sheet: one panel per shot, frame on top, narration underneath. */
export function FramesStage(props: StageProps) {
  const { project, root, busy } = props;
  const enqueue = useEnqueue();
  const [openId, setOpenId] = useState<string | null>(null);
  const open = project.shots.find((s) => s.id === openId);
  const aspect = project.format === 'landscape' ? '16 / 9' : '9 / 16';

  const missing = project.shots.filter((s) => s.takes.length === 0).map((s) => s.id);
  const drawn = project.shots.filter((s) => selectedFile(s)).map((s) => s.id);
  const reviewing = drawn.some((id) => busy.has(`review:${id}`));
  const generate = (shotIds: string[]) => {
    enqueue.mutate({ type: 'shotTakes', projectId: project.id, shotIds });
  };

  return (
    <div className="flex gap-6">
      <div className="flex min-w-0 flex-1 flex-col gap-5">
        <div className="flex items-center gap-2">
          <Button
            variant="primary"
            disabled={missing.length === 0}
            onClick={() => {
              generate(missing);
            }}
          >
            {missing.length > 0 ? `Vẽ ${missing.length} khung hình còn thiếu` : 'Đã đủ khung hình'}
          </Button>
          <Button
            disabled={project.shots.length === 0}
            onClick={() => {
              generate(project.shots.map((s) => s.id));
            }}
          >
            Vẽ lại tất cả
          </Button>
          <Button
            disabled={drawn.length === 0 || reviewing}
            title="Model nhìn ảnh kiểm tra từng khung hình: thiếu hoặc thừa nhân vật, vẽ trùng, có chữ, lỗi tay chân"
            onClick={() => {
              enqueue.mutate({ type: 'review', projectId: project.id, shotIds: drawn });
            }}
          >
            {reviewing ? 'Đang soát…' : 'Soát lỗi khung hình'}
          </Button>
          {enqueue.error && <ErrorText>{errorMessage(enqueue.error)}</ErrorText>}
        </div>

        <ol
          className={clsx(
            'grid gap-x-5 gap-y-8',
            project.format === 'landscape'
              ? 'grid-cols-[repeat(auto-fill,minmax(300px,1fr))]'
              : 'grid-cols-[repeat(auto-fill,minmax(190px,1fr))]',
          )}
        >
          {project.shots.map((shot) => {
            const file = selectedFile(shot);
            const isBusy = busy.has(`shot:${shot.id}`);
            const review = currentReview(shot);
            const isOpen = shot.id === openId;
            return (
              <li key={shot.id}>
                <button
                  type="button"
                  aria-expanded={isOpen}
                  onClick={() => {
                    setOpenId(isOpen ? null : shot.id);
                  }}
                  className="group flex w-full flex-col gap-2 text-left"
                >
                  <div
                    className={clsx(
                      'relative w-full rounded-sm border bg-rail',
                      isOpen ? 'border-marker' : 'border-line group-hover:border-dim',
                    )}
                    style={{ aspectRatio: aspect }}
                  >
                    {file ? (
                      <img
                        src={projectFileUrl(root, project.id, file)}
                        alt={shot.title}
                        className="size-full rounded-sm object-cover"
                      />
                    ) : (
                      <span className="absolute inset-0 flex items-center justify-center p-4 text-center text-[12px] text-faint">
                        {shot.framing}
                      </span>
                    )}
                    {isBusy && (
                      <span className="absolute right-2 bottom-2 rounded bg-rail/90 px-2 py-0.5 text-[12px] text-marker">
                        Đang vẽ…
                      </span>
                    )}
                    {review && !isBusy && (
                      <span
                        className={clsx(
                          'absolute top-2 right-2 flex items-center gap-1 rounded bg-rail/90 px-1.5 py-0.5 text-[12px]',
                          review.ok ? 'text-ok' : 'text-alert',
                        )}
                      >
                        {review.ok ? (
                          <Check size={13} aria-hidden />
                        ) : (
                          <TriangleAlert size={13} aria-hidden />
                        )}
                        {review.ok ? 'Ổn' : `${String(review.issues.length)} lỗi`}
                      </span>
                    )}
                  </div>
                  {/* Caption lines, the way a printed storyboard sheet leaves room under each panel. */}
                  <div className="flex items-baseline gap-2">
                    <span className="text-[12px] text-faint tabular-nums">{shot.id}</span>
                    <span className="truncate font-medium group-hover:underline">{shot.title}</span>
                  </div>
                  <p className="line-clamp-3 font-mono text-[12.5px] leading-relaxed text-dim">
                    {shot.narration}
                  </p>
                </button>
              </li>
            );
          })}
        </ol>
      </div>

      {open && (
        <ShotInspector
          key={open.id}
          {...props}
          shot={open}
          aspect={aspect}
          busy={busy.has(`shot:${open.id}`)}
          onClose={() => {
            setOpenId(null);
          }}
        />
      )}
    </div>
  );
}

interface InspectorProps extends Omit<StageProps, 'busy'> {
  shot: Shot;
  aspect: string;
  busy: boolean;
  onClose: () => void;
}

function ShotInspector({ project, root, shot, aspect, busy, onClose }: InspectorProps) {
  const { draft, set, dirty, save, saving, error } = useShotDraft(project.id, shot);
  const enqueue = useEnqueue();
  const patch = usePatchProject(project.id);

  async function regenerate() {
    if (dirty) await save();
    enqueue.mutate({ type: 'shotTakes', projectId: project.id, shotIds: [shot.id] });
  }

  const shownError = error ?? enqueue.error ?? patch.error;
  const review = currentReview(shot);

  return (
    <aside
      aria-label={`Shot ${shot.id}`}
      className="sticky top-36 flex max-h-[calc(100vh-10rem)] w-[400px] shrink-0 flex-col gap-4 self-start overflow-y-auto rounded-md border border-line bg-sheet p-4"
    >
      <div className="flex items-center justify-between">
        <h2 className="font-medium">
          <span className="mr-2 text-faint tabular-nums">{shot.id}</span>
          {shot.title}
        </h2>
        <button type="button" aria-label="Đóng" onClick={onClose} className="text-faint hover:text-ink">
          <X size={16} />
        </button>
      </div>

      {shot.takes.length > 0 && (
        <TakeStrip
          label={`Các lần vẽ shot ${shot.id}`}
          takes={shot.takes}
          selected={shot.selectedTake}
          root={root}
          projectId={project.id}
          aspect={aspect}
          onSelect={(takeId) => {
            patch.mutate({ type: 'selectTake', target: { kind: 'shot', id: shot.id }, takeId });
          }}
        />
      )}

      {review && !review.ok && (
        <div
          role="status"
          className="flex flex-col gap-1 rounded-md border border-alert/50 px-3 py-2 text-[13px]"
        >
          <p className="font-medium text-alert">Soát lỗi thấy:</p>
          <ul className="list-disc pl-4 text-dim">
            {review.issues.map((issue) => (
              <li key={issue}>{issue}</li>
            ))}
          </ul>
          <p className="text-[12px] text-faint">Sửa mô tả hoặc nhân vật trong khung rồi vẽ lại.</p>
        </div>
      )}
      <ShotFields project={project} draft={draft} onChange={set} />
      {shownError && <ErrorText>{errorMessage(shownError)}</ErrorText>}
      <div className="flex gap-2">
        <Button
          variant="primary"
          disabled={busy || enqueue.isPending || saving}
          onClick={() => void regenerate()}
        >
          {busy ? 'Đang vẽ…' : dirty ? 'Lưu và vẽ lại' : 'Vẽ lại'}
        </Button>
        <Button disabled={!dirty || saving} onClick={() => void save()}>
          Lưu
        </Button>
      </div>
    </aside>
  );
}
