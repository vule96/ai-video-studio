import type { MotionClip } from './bindings/MotionClip';
import type { Shot } from './bindings/Shot';

/** The AI clip the film will use, or null. Mirrors `Shot::fresh_clip` in model.rs. */
export function freshClip(shot: Shot): MotionClip | null {
  const { clip } = shot;
  if (!clip || shot.animation === 'camera') return null;
  const matches =
    clip.take === shot.selectedTake && clip.action === shot.action && clip.hd === (shot.animation === 'aiHd');
  return matches ? clip : null;
}

/** Shots set to AI motion that have a frame but no matching clip yet. */
export function needsClip(shot: Shot): boolean {
  return shot.animation !== 'camera' && shot.selectedTake !== null && freshClip(shot) === null;
}
