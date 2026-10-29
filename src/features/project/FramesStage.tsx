import clsx from 'clsx';
import { Check, TriangleAlert, X } from 'lucide-react';
import { useState } from 'react';

import type { Shot } from '@/lib/bindings/Shot';
import { errorMessage, projectFileUrl } from '@/lib/ipc';
import { useEnqueue, usePatchProject } from '@/lib/queries';
import { Button, ErrorText, Toolbar } from '@/ui';

import type { StageProps } from './ProjectScreen';
import { ShotFields, useShotDraft } from './ShotFields';
import { TakeStrip } from './TakeStrip';

function selectedFile(shot: Shot): string | undefined {
  return shot.takes.find((t) => t.id === shot.selectedTake)?.file;
}

/** New takes drawn per flagged shot by "Tự sửa"; the shot switches only to one that passes. */
const FIX_ATTEMPTS = 3;

/** Review of the selected take; each take keeps its own, so switching takes shows the matching one. */
export function currentReview(shot: Shot) {
  return shot.takes.find((t) => t.id === shot.selectedTake)?.review ?? null;
}

/** The storyboard sheet: one panel per shot, frame on top, narration underneath. */
export function FramesStage(props: StageProps) {
  const { project, root, busy } = props;
  const enqueue = useEnqueue();
  const [openId, setOpenId] = useState<string | null>(null);
  const open = project.shots.find((s) => s.id === openId);
  const aspect = project.format === 'landscape' ? '16 / 9' : '9 / 16';

  const missing = project.shots.filter((s) => s.takes.length === 0).map((s) => s.id);
  // Only frames nobody has looked at yet: after fixing a few shots, re-reviewing costs only those.
  const unreviewed = project.shots.filter((s) => selectedFile(s) && !currentReview(s)).map((s) => s.id);
  const flagged = project.shots.filter((s) => currentReview(s)?.ok === false).map((s) => s.id);
  const reviewing = project.shots.some((s) => busy.has(`review:${s.id}`));
  const fixing = flagged.some((id) => busy.has(`shot:${id}`));
  const generate = (shotIds: string[]) => {
    enqueue.mutate({ type: 'shotTakes', projectId: project.id, shotIds });
  };

  const drawn = project.shots.filter((s) => selectedFile(s)).length;
  const reviewAll = () => {
    const ids = project.shots.filter((s) => selectedFile(s)).map((s) => s.id);
    enqueue.mutate({ type: 'review', projectId: project.id, shotIds: ids, again: true });
  };

  return (
    <div className="flex gap-6">
      <div className="flex min-w-0 flex-1 flex-col gap-5">
        <Toolbar
          actions={
            <>
              {missing.length > 0 && (
                <Button
                  variant="primary"
                  onClick={() => {
                    generate(missing);
                  }}
                >
                  Vẽ {missing.length} khung còn thiếu
                </Button>
              )}
              {flagged.length > 0 && (
                <Button
                  variant={missing.length > 0 ? 'quiet' : 'primary'}
                  disabled={fixing}
                  title={`Mỗi cảnh lỗi nặng vẽ thêm ${String(FIX_ATTEMPTS)} ảnh và soát từng ảnh; chỉ đổi sang ảnh mới khi ảnh đó đạt. Ảnh mới vẫn nằm trong dải ảnh để chọn tay.`}
                  onClick={() => {
                    enqueue.mutate({
                      type: 'fix',
                      projectId: project.id,
                      shotIds: flagged,
                      attempts: FIX_ATTEMPTS,
                    });
                  }}
                >
                  {fixing ? 'Đang tự sửa…' : `Tự sửa ${String(flagged.length)} cảnh lỗi`}
                </Button>
              )}
              {unreviewed.length > 0 ? (
                <Button
                  disabled={reviewing}
                  title="Model nhìn ảnh kiểm tra từng khung: lỗi nặng (sai loài, nhân vật thiếu hoặc bị vẽ hai lần, sai trang phục chính, có chữ) và ghi chú nhỏ."
                  onClick={() => {
                    enqueue.mutate({
                      type: 'review',
                      projectId: project.id,
                      shotIds: unreviewed,
                      again: false,
                    });
                  }}
                >
                  {reviewing ? 'Đang soát…' : `Soát ${String(unreviewed.length)} khung chưa soát`}
                </Button>
              ) : (
                <Button
                  disabled={reviewing || drawn === 0}
                  title="Soát lại cả những khung đã soát, ví dụ sau khi app đổi cách soát."
                  onClick={reviewAll}
                >
                  {reviewing ? 'Đang soát…' : 'Soát lại tất cả'}
                </Button>
              )}
              <Button
                disabled={project.shots.length === 0}
                onClick={() => {
                  generate(project.shots.map((s) => s.id));
                }}
              >
                Vẽ lại tất cả
              </Button>
            </>
          }
        >
          <span className="text-[15px] font-semibold text-ink">
            {drawn}/{project.shots.length} khung đã vẽ
          </span>
          {flagged.length > 0 && <span className="ml-2 text-alert">{flagged.length} cảnh có lỗi nặng</span>}
          {enqueue.error && <ErrorText>{errorMessage(enqueue.error)}</ErrorText>}
        </Toolbar>

        <ol
          className={clsx(
            'grid gap-x-5 gap-y-7',
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
                      'relative w-full rounded bg-well ring-1',
                      isOpen ? 'ring-2 ring-marker' : 'ring-line group-hover:ring-edge',
                    )}
                    style={{ aspectRatio: aspect }}
                  >
                    {file ? (
                      <img
                        src={projectFileUrl(root, project.id, file)}
                        alt={shot.title}
                        className="size-full rounded object-cover"
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
                  <p className="line-clamp-3 text-[13px] leading-relaxed text-dim">{shot.narration}</p>
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
      className="sticky top-[176px] flex max-h-[calc(100vh-200px)] w-[420px] shrink-0 flex-col self-start rounded-md bg-sheet"
    >
      {/* Fields scroll; the action row below stays in reach. */}
      <div className="flex min-h-0 flex-col gap-4 overflow-y-auto p-5">
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
            <p className="font-medium text-alert">Lỗi nặng</p>
            <ul className="list-disc pl-4 text-dim">
              {review.issues.map((issue) => (
                <li key={issue}>{issue}</li>
              ))}
            </ul>
            <p className="text-[12px] text-faint">
              Lỗi lặp ở nhiều cảnh thì sửa ngoại hình ở bước Nhân vật. Lỗi ngẫu nhiên thì bấm Tự sửa: app vẽ{' '}
              {FIX_ATTEMPTS} ảnh mới và chỉ đổi khi có ảnh đạt.
            </p>
          </div>
        )}
        {review && review.notes.length > 0 && (
          <details className="text-[13px]">
            <summary className="cursor-pointer text-dim">
              {review.ok ? 'Soát: ổn. ' : ''}
              {review.notes.length} ghi chú nhỏ (không tính là lỗi)
            </summary>
            <ul className="mt-1.5 list-disc pl-4 text-faint">
              {review.notes.map((note) => (
                <li key={note}>{note}</li>
              ))}
            </ul>
          </details>
        )}
        <ShotFields project={project} draft={draft} onChange={set} />
        {shownError && <ErrorText>{errorMessage(shownError)}</ErrorText>}
      </div>
      <div className="flex shrink-0 gap-2 border-t border-line px-5 py-3">
        <Button
          variant="primary"
          disabled={busy || enqueue.isPending || saving}
          onClick={() => void regenerate()}
        >
          {busy ? 'Đang vẽ…' : dirty ? 'Lưu và vẽ lại' : 'Vẽ lại'}
        </Button>
        {review && !review.ok && (
          <Button
            disabled={busy || enqueue.isPending || saving}
            onClick={() => {
              void (async () => {
                if (dirty) await save();
                enqueue.mutate({
                  type: 'fix',
                  projectId: project.id,
                  shotIds: [shot.id],
                  attempts: FIX_ATTEMPTS,
                });
              })();
            }}
          >
            Tự sửa
          </Button>
        )}
        <Button disabled={!dirty || saving} onClick={() => void save()}>
          Lưu
        </Button>
      </div>
    </aside>
  );
}
