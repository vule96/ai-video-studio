import { useMutation, useQueryClient } from '@tanstack/react-query';
import { Check, CircleAlert, Music } from 'lucide-react';
import { useState } from 'react';

import type { Project } from '@/lib/bindings/Project';
import { api, errorMessage, projectFileUrl } from '@/lib/ipc';
import { isActive, useJobs } from '@/lib/jobs';
import { keys, useEnqueue, useHealth } from '@/lib/queries';
import { Button, ErrorText, Field, Input, Progress, Select } from '@/ui';

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

  return (
    <div className="flex gap-8">
      <div className="min-w-0 flex-1">
        {video ? (
          <div className="flex flex-col gap-2">
            {/* key: a new render is a new file, so the player reloads instead of keeping the old one. */}
            <video
              key={video.file}
              controls
              preload="metadata"
              src={projectFileUrl(root, project.id, video.file)}
              className="w-full rounded-sm border border-line bg-rail"
              style={{ aspectRatio: aspect, maxHeight: 'calc(100vh - 14rem)' }}
            />
            <p className="text-[12px] text-faint">
              {formatDuration(video.durationMs)}, giọng {video.voice}
              {video.burnedSubtitles ? ', phụ đề in trong hình' : ''}. File:{' '}
              <span className="font-mono select-all">
                {[root, project.id, video.file].join(root.includes('\\') ? '\\' : '/')}
              </span>{' '}
              (phụ đề .srt cùng tên)
            </p>
          </div>
        ) : (
          <div
            className="flex w-full items-center justify-center rounded-sm border border-dashed border-line p-6 text-center text-dim"
            style={{ aspectRatio: aspect, maxHeight: 'calc(100vh - 14rem)' }}
          >
            Chưa dựng. Phim gồm các khung hình đã chọn, chuyển động theo gợi ý camera của từng shot, đọc lời
            dẫn bằng VieNeu-TTS.
          </div>
        )}
      </div>

      <aside className="flex w-[320px] shrink-0 flex-col gap-4 self-start">
        <Field
          label="Giọng đọc"
          hint={
            voices.length > 0
              ? 'Lời dẫn của mọi shot đọc bằng giọng này.'
              : 'Chưa kết nối VieNeu-TTS, gõ tên giọng.'
          }
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

        <label className="flex items-start gap-2 text-[13px]">
          <input
            id="burn-subtitles"
            type="checkbox"
            className="mt-0.5 accent-marker"
            checked={burn}
            onChange={(e) => {
              setBurn(e.target.checked);
            }}
          />
          <span>
            In phụ đề vào hình
            <span className="block text-[12px] text-faint">
              Nên bật cho TikTok, Reels, Shorts. File .srt riêng luôn có.
            </span>
          </span>
        </label>

        <ul className="flex flex-col gap-2 text-[13px]">
          <Item ok={missing.length === 0}>
            {missing.length === 0
              ? `Đủ ${String(project.shots.length)} khung hình`
              : `Shot ${missing.join(', ')} chưa có khung hình`}
          </Item>
          {flagged.length > 0 && (
            <Item ok={false} hint="Xem chi tiết ở bước Khung hình. Vẫn dựng được nếu bạn thấy ổn.">
              {`Soát lỗi: ${flagged.map((s) => s.id).join(', ')} có cảnh báo`}
            </Item>
          )}
          {silent > 0 && (
            <Item ok>{`${String(silent)} shot không có lời dẫn, sẽ giữ 3 giây không tiếng`}</Item>
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
            className="self-start"
            disabled={missing.length === 0 ? enqueue.isPending || voice.trim() === '' : true}
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
      </aside>
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
      <span className="text-[13px] text-dim">Nhạc nền</span>
      <div className="flex items-center gap-2">
        <label className="inline-flex h-8 min-w-0 flex-1 cursor-pointer items-center gap-1.5 rounded-md border border-line px-2.5 text-[13px] focus-within:border-dim hover:border-dim">
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
