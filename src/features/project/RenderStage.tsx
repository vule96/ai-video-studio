import { useMutation, useQueryClient } from '@tanstack/react-query';
import { Check, CircleAlert, Music } from 'lucide-react';
import { useState } from 'react';

import type { Project } from '@/lib/bindings/Project';
import { api, errorMessage, projectFileUrl } from '@/lib/ipc';
import { isActive, useJobs } from '@/lib/jobs';
import { keys, useEnqueue, useHealth } from '@/lib/queries';
import { Button, Checkbox, ErrorText, Field, Input, Panel, Progress, Select } from '@/ui';

import { currentReview } from './FramesStage';
import type { StageProps } from './ProjectScreen';

/** A storytelling voice suits documentaries; any installed voice can be picked instead. */
const DEFAULT_VOICE = 'Thiện Minh';

function formatDuration(ms: number): string {
  const s = Math.round(ms / 1000);
  return `${String(Math.floor(s / 60))}:${String(s % 60).padStart(2, '0')}`;
}

/** Final step: narrate every shot, animate the frames, join them into one video. */
export function RenderStage({ project, root, busy }: StageProps) {
  const health = useHealth();
  const enqueue = useEnqueue();
  const [voice, setVoice] = useState(project.video?.voice ?? DEFAULT_VOICE);
  // Short vertical videos are mostly watched muted, so they default to burned-in subtitles.
  const [burn, setBurn] = useState(project.video?.burnedSubtitles ?? project.format === 'portrait');
  // Latest render job of this project: shows progress while running, the error if it failed.
  const job = useJobs((s) =>
    [...s.byId.values()].findLast((j) => j.projectId === project.id && j.spec.type === 'render'),
  );

  const voices = health.data?.ttsVoices ?? [];
  const missing = project.shots.filter((s) => !s.takes.some((t) => t.id === s.selectedTake)).map((s) => s.id);
  const silent = project.shots.filter((s) => s.narration.trim() === '').length;
  const flagged = project.shots.filter((s) => currentReview(s)?.ok === false);
  const services = (health.data?.checks ?? []).filter((c) => c.name === 'VieNeu-TTS' || c.name === 'ffmpeg');
  const rendering = busy.has('render');
  const aspect = project.format === 'landscape' ? '16 / 9' : '9 / 16';
  const video = project.video;

  const ready = missing.length === 0 && voice.trim() !== '';

  return (
    <div className="grid grid-cols-[minmax(0,1fr)_360px] items-start gap-8">
      <div className="flex min-w-0 flex-col gap-3">
        {video ? (
          <>
            {/* key: a new render is a new file, so the player reloads instead of keeping the old one. */}
            <video
              key={video.file}
              controls
              preload="metadata"
              src={projectFileUrl(root, project.id, video.file)}
              className="mx-auto w-full rounded bg-black ring-1 ring-line"
              style={{ aspectRatio: aspect, maxHeight: 'calc(100vh - 260px)' }}
            />
            <div className="flex flex-wrap items-start justify-between gap-4">
              <div className="flex min-w-0 flex-col gap-0.5 text-[13px]">
                <p>
                  <span className="font-semibold tabular-nums">{formatDuration(video.durationMs)}</span>
                  <span className="text-dim">
                    {' '}
                    · giọng {video.voice}
                    {video.burnedSubtitles ? ' · phụ đề in trong hình' : ''}
                  </span>
                </p>
                <p className="font-mono text-[12px] break-all text-faint select-all">
                  {[root, project.id, video.file].join(root.includes('\\') ? '\\' : '/')}
                </p>
                <p className="text-[12px] text-faint">Phụ đề .srt nằm cùng thư mục, cùng tên.</p>
              </div>
              <DeleteVideo projectId={project.id} disabled={rendering} />
            </div>
          </>
        ) : (
          <div
            className="flex w-full items-center justify-center rounded border border-dashed border-edge p-8 text-center text-dim"
            style={{ aspectRatio: aspect, maxHeight: 'calc(100vh - 260px)' }}
          >
            <p className="max-w-md">
              Chưa dựng. Phim gồm các khung hình đã chọn, chuyển động theo gợi ý máy quay của từng shot, đọc
              lời dẫn bằng VieNeu-TTS.
            </p>
          </div>
        )}
      </div>

      <aside className="sticky top-[176px] flex flex-col gap-4">
        <Panel title="Âm thanh">
          <Field
            label="Giọng đọc"
            hint={voices.length > 0 ? undefined : 'Chưa kết nối VieNeu-TTS, gõ tên giọng.'}
          >
            {(id) =>
              voices.length > 0 ? (
                <Select
                  id={id}
                  value={voice}
                  onChange={(e) => {
                    setVoice(e.target.value);
                  }}
                >
                  {!voices.some((v) => v.id === voice) && <option value={voice}>{voice} (không có)</option>}
                  {voices.map((v) => (
                    <option key={v.id} value={v.id}>
                      {v.id}
                      {v.description && `, ${v.description}`}
                    </option>
                  ))}
                </Select>
              ) : (
                <Input
                  id={id}
                  value={voice}
                  onChange={(e) => {
                    setVoice(e.target.value);
                  }}
                />
              )
            }
          </Field>
          <MusicField project={project} />
          <Checkbox
            id="burn-subtitles"
            checked={burn}
            onChange={setBurn}
            label="In phụ đề vào hình"
            hint="Nên bật cho TikTok, Reels, Shorts. File .srt riêng luôn có."
          />
        </Panel>

        <Panel title="Kiểm tra trước khi dựng">
          <ul className="flex flex-col gap-2 text-[13px]">
            <Item ok={missing.length === 0}>
              {missing.length === 0
                ? `Đủ ${String(project.shots.length)} khung hình`
                : `Shot ${missing.join(', ')} chưa có khung hình`}
            </Item>
            {flagged.length > 0 && (
              <Item ok={false} hint="Xem ở bước Khung hình. Vẫn dựng được nếu bạn thấy ổn.">
                {`${flagged.map((s) => s.id).join(', ')} có lỗi nặng`}
              </Item>
            )}
            {silent > 0 && (
              <Item ok>{`${String(silent)} shot không có lời dẫn, giữ 3 giây không tiếng`}</Item>
            )}
            {services.map((c) => (
              <Item key={c.name} ok={c.ok} hint={c.hint}>
                {c.name}
              </Item>
            ))}
          </ul>

          {enqueue.error && <ErrorText>{errorMessage(enqueue.error)}</ErrorText>}
          {job?.status === 'failed' && job.error && <ErrorText>{job.error}</ErrorText>}

          {rendering && job && isActive(job) ? (
            <div className="flex flex-col gap-2">
              <Progress done={job.done} total={job.total} />
              <p className="text-[13px] text-dim">
                {job.status === 'queued' ? 'Đang chờ đến lượt…' : job.message}
              </p>
            </div>
          ) : (
            <Button
              variant="primary"
              className="w-full"
              disabled={!ready || enqueue.isPending}
              onClick={() => {
                enqueue.mutate({
                  type: 'render',
                  projectId: project.id,
                  voice: voice.trim(),
                  burnSubtitles: burn,
                });
              }}
            >
              {video ? 'Dựng lại' : 'Dựng phim'}
            </Button>
          )}
          <p className="text-[12px] text-faint">
            Lời dẫn đã đọc được giữ lại: dựng lại sau khi chỉ sửa ảnh hay chuyển động thì không phải chờ đọc
            lại.
          </p>
        </Panel>
      </aside>
    </div>
  );
}

/** Deletes the rendered file after an inline confirm; narration stays cached so a re-render is quick. */
function DeleteVideo({ projectId, disabled }: { projectId: string; disabled: boolean }) {
  const qc = useQueryClient();
  const [confirming, setConfirming] = useState(false);
  const remove = useMutation({
    mutationFn: () => api.projectVideoDelete(projectId),
    onSuccess: (p) => {
      qc.setQueryData(keys.project(p.id), p);
      setConfirming(false);
    },
  });
  return (
    <div className="flex flex-wrap items-center justify-end gap-2 text-[13px]">
      {confirming ? (
        <>
          <span className="text-dim">
            Xoá file video và phụ đề? Lời dẫn đã đọc vẫn giữ để dựng lại nhanh.
          </span>
          <Button
            variant="danger"
            disabled={remove.isPending}
            onClick={() => {
              remove.mutate();
            }}
          >
            Xoá video
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
          variant="danger"
          disabled={disabled}
          onClick={() => {
            setConfirming(true);
          }}
        >
          Xoá video
        </Button>
      )}
      {remove.error && <ErrorText>{errorMessage(remove.error)}</ErrorText>}
    </div>
  );
}

/** Background music: picked once per project, looped and ducked under the narration at render time. */
function MusicField({ project }: { project: Project }) {
  const qc = useQueryClient();
  const onSuccess = (p: Project) => {
    qc.setQueryData(keys.project(p.id), p);
  };
  const set = useMutation({ mutationFn: (file: File) => api.projectMusicSet(project.id, file), onSuccess });
  const remove = useMutation({ mutationFn: () => api.projectMusicRemove(project.id), onSuccess });
  const error = set.error ?? remove.error;
  return (
    <div className="flex flex-col gap-1">
      <span className="text-[12.5px] font-medium text-dim">Nhạc nền</span>
      <div className="flex items-center gap-2">
        <label className="inline-flex h-8 min-w-0 flex-1 cursor-pointer items-center gap-1.5 rounded border border-line bg-well px-2.5 text-[13px] focus-within:border-dim hover:border-edge">
          <Music size={14} className="shrink-0 text-faint" aria-hidden />
          <span className="truncate">
            {set.isPending ? 'Đang chép…' : (project.music?.name ?? 'Chọn file nhạc (mp3, m4a, wav…)')}
          </span>
          <input
            id="music-file"
            type="file"
            accept="audio/*,.mp3,.m4a,.aac,.wav,.ogg,.flac"
            className="sr-only"
            onChange={(e) => {
              const file = e.target.files?.[0];
              if (file) set.mutate(file);
              e.target.value = '';
            }}
          />
        </label>
        {project.music && (
          <Button
            disabled={remove.isPending}
            onClick={() => {
              remove.mutate();
            }}
          >
            Bỏ
          </Button>
        )}
      </div>
      <p className="text-[12px] text-faint">
        Lặp lại cho đủ độ dài, tự nhỏ đi khi có lời dẫn. Chỉ dùng nhạc bạn có quyền dùng.
      </p>
      {error && <ErrorText>{errorMessage(error)}</ErrorText>}
    </div>
  );
}

function Item({ ok, hint, children }: { ok: boolean; hint?: string; children: string }) {
  return (
    <li className="flex gap-2">
      {ok ? (
        <Check size={16} className="mt-0.5 shrink-0 text-ok" aria-label="Ổn" />
      ) : (
        <CircleAlert size={16} className="mt-0.5 shrink-0 text-alert" aria-label="Chưa ổn" />
      )}
      <div className="min-w-0">
        <p>{children}</p>
        {!ok && hint && <p className="text-[12px] whitespace-pre-wrap text-faint">{hint}</p>}
      </div>
    </li>
  );
}
