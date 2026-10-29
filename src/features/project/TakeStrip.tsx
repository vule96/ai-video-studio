import clsx from 'clsx';

import type { Take } from '@/lib/bindings/Take';
import { projectFileUrl } from '@/lib/ipc';

interface Props {
  takes: Take[];
  selected: string | null;
  onSelect: (takeId: string) => void;
  root: string;
  projectId: string;
  /** CSS aspect ratio of the images, e.g. "2 / 3". */
  aspect: string;
  label: string;
  /** Tailwind width class of each thumbnail. */
  itemWidth?: string;
}

/**
 * All generated takes of a character or shot, newest last. The chosen one is "circled",
 * like the take a director circles on the camera report.
 */
export function TakeStrip({
  takes,
  selected,
  onSelect,
  root,
  projectId,
  aspect,
  label,
  itemWidth = 'w-28',
}: Props) {
  if (takes.length === 0) return null;
  return (
    <div role="radiogroup" aria-label={label} className="flex gap-3 overflow-x-auto pt-1.5 pb-2 pl-1.5">
      {takes.map((t, i) => {
        const chosen = t.id === selected;
        return (
          <button
            key={t.id}
            type="button"
            role="radio"
            aria-checked={chosen}
            aria-label={`Ảnh ${i + 1}`}
            title={`Seed ${t.seed}`}
            onClick={() => {
              onSelect(t.id);
            }}
            className={clsx('relative shrink-0', itemWidth)}
            style={{ aspectRatio: aspect }}
          >
            <img
              src={projectFileUrl(root, projectId, t.file)}
              alt=""
              loading="lazy"
              className={clsx('size-full rounded object-cover', !chosen && 'opacity-60 hover:opacity-100')}
            />
            {chosen && <CircleMark />}
          </button>
        );
      })}
    </div>
  );
}

/** A hand-drawn china-marker loop around the chosen take. */
function CircleMark() {
  return (
    <svg
      aria-hidden
      viewBox="0 0 100 100"
      preserveAspectRatio="none"
      className="pointer-events-none absolute -inset-1.5 size-[calc(100%+12px)] text-marker"
    >
      <path
        d="M52 3 C80 2 97 14 97 48 C97 84 78 97 48 97 C18 97 3 82 3 50 C3 20 20 5 47 4 C60 3 70 6 74 9"
        fill="none"
        stroke="currentColor"
        strokeWidth="2.6"
        strokeLinecap="round"
        vectorEffect="non-scaling-stroke"
      />
    </svg>
  );
}
