import clsx from 'clsx';

import type { Animation } from '@/lib/bindings/Animation';
import type { Project } from '@/lib/bindings/Project';
import type { Shot } from '@/lib/bindings/Shot';
import { useDraft } from '@/lib/draft';
import { usePatchProject } from '@/lib/queries';
import { Field, Input, Segmented, TextArea } from '@/ui';

type Draft = Pick<
  Shot,
  'title' | 'characters' | 'framing' | 'prompt' | 'narration' | 'motion' | 'animation' | 'action'
>;

/** Same limit as `MAX_CHARACTERS_IN_SHOT` in model.rs: more references and the model mixes faces up. */
const MAX_IN_SHOT = 3;

const ANIMATIONS = [
  { value: 'camera', label: 'Zoom/pan' },
  { value: 'ai', label: 'AI' },
  { value: 'aiHd', label: 'AI 720p thật' },
] as const satisfies readonly { value: Animation; label: string }[];

/** Times measured on an RTX 5060 Ti 16 GB (README, v0.3). */
const ANIMATION_HINTS: Record<Animation, string> = {
  camera: 'Ảnh tĩnh, máy quay zoom hoặc lia theo gợi ý bên dưới. Không tốn thời gian.',
  ai: 'Wan 2.2 cho nhân vật cử động 5 giây: vẽ 480p, phóng lên 720p, mượt 32 fps. Khoảng 4 phút mỗi cảnh.',
  aiHd: 'Vẽ thẳng 720p: nét hơn chút nhưng khoảng 11 phút mỗi cảnh và máy chạy nặng. Nên chạy lúc không dùng máy.',
};

function draftOf(shot: Shot): Draft {
  const { title, characters, framing, prompt, narration, motion, animation, action } = shot;
  return { title, characters, framing, prompt, narration, motion, animation, action };
}

/** Local edit state for one shot. `save` patches only this shot, so takes added meanwhile survive. */
export function useShotDraft(projectId: string, shot: Shot) {
  const patch = usePatchProject(projectId);
  const { draft, dirty, set, reset } = useDraft(draftOf(shot));
  const save = async () => {
    await patch.mutateAsync({ type: 'shot', id: shot.id, ...draft });
    reset();
  };
  return { draft, set, dirty, save, saving: patch.isPending, error: patch.error };
}

interface Props {
  project: Project;
  draft: Draft;
  onChange: (patch: Partial<Draft>) => void;
  compact?: boolean;
}

/** Shot fields. Prompt and framing are English (they go to the image model); narration is in the project language. */
export function ShotFields({ project, draft, onChange, compact = false }: Props) {
  const set = <K extends keyof Draft>(key: K, value: Draft[K]) => {
    onChange({ [key]: value });
  };
  const toggle = (id: string) => {
    // Keep the project's character order so reference-image numbering stays stable.
    const next = draft.characters.includes(id)
      ? draft.characters.filter((c) => c !== id)
      : [...draft.characters, id];
    set(
      'characters',
      project.characters.map((c) => c.id).filter((c) => next.includes(c)),
    );
  };

  return (
    <div className="flex flex-col gap-3">
      <div className="grid grid-cols-2 gap-3">
        <Field label="Tên shot">
          {(id) => (
            <Input
              id={id}
              value={draft.title}
              onChange={(e) => {
                set('title', e.target.value);
              }}
            />
          )}
        </Field>
        <Field label="Góc máy">
          {(id) => (
            <Input
              id={id}
              value={draft.framing}
              placeholder="medium two-shot"
              onChange={(e) => {
                set('framing', e.target.value);
              }}
            />
          )}
        </Field>
      </div>

      {project.characters.length > 0 && (
        <fieldset className="flex flex-col gap-1">
          <legend className="mb-1.5 text-[12.5px] font-medium text-dim">
            Nhân vật trong khung <span className="text-faint">(tối đa {MAX_IN_SHOT})</span>
          </legend>
          <div className="flex flex-wrap gap-1.5">
            {project.characters.map((c) => {
              const on = draft.characters.includes(c.id);
              return (
                <button
                  key={c.id}
                  type="button"
                  aria-pressed={on}
                  disabled={!on && draft.characters.length >= MAX_IN_SHOT}
                  onClick={() => {
                    toggle(c.id);
                  }}
                  className={clsx(
                    'rounded-full border px-2.5 py-0.5 text-[13px] disabled:cursor-not-allowed disabled:opacity-40',
                    on ? 'border-marker text-ink' : 'border-line text-faint hover:text-ink',
                  )}
                >
                  {c.name}
                </button>
              );
            })}
          </div>
        </fieldset>
      )}

      <Field label="Mô tả khung hình (tiếng Anh, gửi cho model ảnh)">
        {(id) => (
          <TextArea
            id={id}
            rows={compact ? 3 : 4}
            className="font-mono text-[12.5px]"
            value={draft.prompt}
            onChange={(e) => {
              set('prompt', e.target.value);
            }}
          />
        )}
      </Field>
      <Field label="Lời dẫn">
        {(id) => (
          <TextArea
            id={id}
            rows={2}
            className="text-[13.5px]"
            value={draft.narration}
            onChange={(e) => {
              set('narration', e.target.value);
            }}
          />
        )}
      </Field>
      <fieldset className="flex flex-col gap-1.5">
        <legend className="mb-1.5 text-[12.5px] font-medium text-dim">Chuyển động trong phim</legend>
        <Segmented
          label="Chuyển động trong phim"
          value={draft.animation}
          options={ANIMATIONS}
          onChange={(v) => {
            set('animation', v);
          }}
        />
        <p className="text-[12px] leading-snug text-faint">{ANIMATION_HINTS[draft.animation]}</p>
      </fieldset>
      {draft.animation !== 'camera' && (
        <Field
          label="Hành động (tiếng Anh, gửi cho model video)"
          hint="Ai làm gì trong 5 giây: bước chân, tay, quay đầu; gió, khói, nước. Tả càng cụ thể nhân vật càng cử động rõ."
        >
          {(id) => (
            <TextArea
              id={id}
              rows={2}
              className="font-mono text-[12.5px]"
              value={draft.action}
              placeholder="She steps forward and raises her lantern, her cloak swaying in the wind."
              onChange={(e) => {
                set('action', e.target.value);
              }}
            />
          )}
        </Field>
      )}
      <Field
        label="Chuyển động máy quay"
        hint={
          draft.animation === 'camera'
            ? 'push-in, pull-out, pan left/right, tilt up/down, static, shake; thêm slow hoặc fast để đổi tốc độ.'
            : 'Cũng gửi cho model video, ví dụ slow push-in, tracking shot, static.'
        }
      >
        {(id) => (
          <Input
            id={id}
            value={draft.motion}
            placeholder="slow push-in"
            onChange={(e) => {
              set('motion', e.target.value);
            }}
          />
        )}
      </Field>
    </div>
  );
}
