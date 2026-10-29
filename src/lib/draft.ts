import { useState } from 'react';

/** True when any edited field differs from the saved value (arrays compared by content). */
export function isDirty<T extends object>(base: T, edits: Partial<T>): boolean {
  return (Object.keys(edits) as (keyof T)[]).some(
    (k) => JSON.stringify(edits[k]) !== JSON.stringify(base[k]),
  );
}

/**
 * Form state for data that a background job may change while the form is open.
 * Only the fields the user touched are stored; everything else follows `base`, so a form
 * opened before the data arrived (or before a job updated it) never shows stale values.
 */
export function useDraft<T extends object>(base: T) {
  const [edits, setEdits] = useState<Partial<T>>({});
  return {
    draft: { ...base, ...edits },
    dirty: isDirty(base, edits),
    set: (patch: Partial<T>) => {
      setEdits((e) => ({ ...e, ...patch }));
    },
    reset: () => {
      setEdits({});
    },
  };
}
