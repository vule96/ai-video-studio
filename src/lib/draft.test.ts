import { describe, expect, it } from 'vitest';

import { isDirty } from './draft';

describe('isDirty', () => {
  const base = { title: 'A', characters: ['toru', 'naoko'] };

  it('is clean when edits match the saved values', () => {
    expect(isDirty(base, {})).toBe(false);
    expect(isDirty(base, { title: 'A', characters: ['toru', 'naoko'] })).toBe(false);
  });

  it('detects changed fields, including array content', () => {
    expect(isDirty(base, { title: 'B' })).toBe(true);
    expect(isDirty(base, { characters: ['naoko', 'toru'] })).toBe(true);
  });
});
