import { describe, expect, it } from 'vitest';

import type { Shot } from './bindings/Shot';
import { freshClip, needsClip } from './clip';

const shot: Shot = {
  id: 'S01',
  title: '',
  characters: [],
  framing: '',
  prompt: '',
  narration: '',
  motion: '',
  takes: [],
  selectedTake: 't1',
  animation: 'ai',
  action: 'walks',
  clip: { file: 'clips/a.mp4', take: 't1', action: 'walks', hd: false, createdAt: 0 },
};

describe('freshClip', () => {
  it('keeps a clip only while it matches the shot', () => {
    expect(freshClip(shot)).not.toBeNull();
    expect(needsClip(shot)).toBe(false);
    expect(freshClip({ ...shot, animation: 'aiHd' })).toBeNull();
    expect(freshClip({ ...shot, selectedTake: 't2' })).toBeNull();
    expect(freshClip({ ...shot, action: 'runs' })).toBeNull();
    expect(needsClip({ ...shot, action: 'runs' })).toBe(true);
  });

  it('never asks for a clip on camera shots or shots without a frame', () => {
    expect(needsClip({ ...shot, animation: 'camera', clip: null })).toBe(false);
    expect(needsClip({ ...shot, selectedTake: null, clip: null })).toBe(false);
  });
});
