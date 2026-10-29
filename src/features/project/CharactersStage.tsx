import type { Character } from '@/lib/bindings/Character';
import { useDraft } from '@/lib/draft';
import { errorMessage } from '@/lib/ipc';
import { useEnqueue, usePatchProject } from '@/lib/queries';
import { Button, ErrorText, Field, Input, TextArea, Toolbar } from '@/ui';

import type { StageProps } from './ProjectScreen';
import { TakeStrip } from './TakeStrip';

const CANDIDATES = 3;

/** Model sheet: each character's fixed look, its generated portraits, and the one used as reference. */
export function CharactersStage({ project, root, busy }: StageProps) {
  if (project.characters.length === 0) {
    return (
      <p className="text-dim">Storyboard này không có nhân vật lặp lại. Có thể sang thẳng bước Khung hình.</p>
    );
  }
  return (
    <div className="flex flex-col gap-4">
      <Toolbar>
        Ảnh được khoanh là ảnh tham chiếu: mọi khung hình có nhân vật này vẽ theo đúng mặt, dáng và trang phục
        trong ảnh đó. Đổi ảnh tham chiếu thì vẽ lại các khung hình có nhân vật.
      </Toolbar>
      {project.characters.map((c) => (
        <CharacterRow
          key={c.id}
          character={c}
          projectId={project.id}
          root={root}
          busy={busy.has(`character:${c.id}`)}
        />
      ))}
    </div>
  );
}

interface RowProps {
  character: Character;
  projectId: string;
  root: string;
  busy: boolean;
}

function CharacterRow({ character: c, projectId, root, busy }: RowProps) {
  const patch = usePatchProject(projectId);
  const enqueue = useEnqueue();
  const { draft, dirty, set, reset } = useDraft({ name: c.name, description: c.description });
  const save = async () => {
    await patch.mutateAsync({ type: 'character', id: c.id, ...draft });
    reset();
  };

  async function generate() {
    // Generate from what is on screen, not from the last saved text.
    if (dirty) await save();
    enqueue.mutate({ type: 'characterTakes', projectId, characterId: c.id, count: CANDIDATES });
  }

  const error = patch.error ?? enqueue.error;

  return (
    <section
      aria-label={c.name}
      className="grid grid-cols-[360px_minmax(0,1fr)] gap-8 rounded-md bg-sheet p-5"
    >
      <div className="flex flex-col gap-3">
        <Field label="Tên">
          {(id) => (
            <Input
              id={id}
              value={draft.name}
              onChange={(e) => {
                set({ name: e.target.value });
              }}
            />
          )}
        </Field>
        <Field label="Ngoại hình cố định (tiếng Anh)" hint="Tuổi, tóc, gương mặt, một bộ trang phục kèm màu.">
          {(id) => (
            <TextArea
              id={id}
              rows={6}
              className="font-mono text-[12.5px]"
              value={draft.description}
              onChange={(e) => {
                set({ description: e.target.value });
              }}
            />
          )}
        </Field>
        {error && <ErrorText>{errorMessage(error)}</ErrorText>}
        <div className="flex gap-2">
          <Button variant="primary" disabled={busy || enqueue.isPending} onClick={() => void generate()}>
            {busy
              ? 'Đang sinh ảnh…'
              : c.takes.length
                ? `Sinh thêm ${CANDIDATES} ảnh`
                : `Sinh ${CANDIDATES} ảnh`}
          </Button>
          <Button disabled={!dirty || patch.isPending} onClick={() => void save()}>
            Lưu
          </Button>
        </div>
      </div>

      <div className="min-w-0">
        {c.takes.length === 0 ? (
          <div className="flex h-full min-h-40 items-center justify-center rounded-md border border-dashed border-edge px-5 text-[13px] text-faint">
            {busy ? 'Đang vẽ…' : 'Chưa có ảnh. Sinh vài phương án rồi khoanh ảnh ưng nhất.'}
          </div>
        ) : (
          <TakeStrip
            label={`Ảnh của ${c.name}`}
            takes={c.takes}
            selected={c.selectedTake}
            root={root}
            projectId={projectId}
            aspect="832 / 1216"
            itemWidth="w-40"
            onSelect={(takeId) => {
              patch.mutate({ type: 'selectTake', target: { kind: 'character', id: c.id }, takeId });
            }}
          />
        )}
      </div>
    </section>
  );
}
