import clsx from 'clsx';
import { Check, TriangleAlert } from 'lucide-react';

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
    // shrink-0: in a scrolling column (the shot inspector) an overflow box would otherwise be squeezed to nothing.
    <div
      role="radiogroup"
      aria-label={label}
      className="flex shrink-0 gap-3 overflow-x-auto pt-1.5 pb-2 pl-1.5"
    >
      {takes.map((t, i) => {
        const chosen = t.id === selected;
        return (
          <button
            key={t.id}
            type="button"
            role="radio"
            aria-checked={chosen}
            aria-label={`Ảnh ${String(i + 1)}${t.review ? (t.review.ok ? ', soát: ổn' : `, soát: ${String(t.review.issues.length)} lỗi`) : ''}`}
            title={t.review && !t.review.ok ? t.review.issues.join('\n') : `Seed ${String(t.seed)}`}
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
            {t.review && (
              <span
                className={clsx(
                  'absolute top-1 right-1 flex items-center gap-0.5 rounded bg-rail/90 px-1 text-[11px]',
                  t.review.ok ? 'text-ok' : 'text-alert',
                )}
                aria-hidden
              >
                {t.review.ok ? <Check size={11} /> : <TriangleAlert size={11} />}
                {!t.review.ok && t.review.issues.length}
              </span>
            )}
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
