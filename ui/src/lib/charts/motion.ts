// The one place a chart's motion is timed. A person who asked for less motion gets none, and a
// test never asserts a value half-way through a tween (`decisions.md#ui`, 2026-10-11).
import { cubicOut } from 'svelte/easing';

function still(): boolean {
  if (import.meta.env.MODE === 'test') return true;
  try {
    return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  } catch {
    return true;
  }
}

/** How long a move takes, in milliseconds: nothing when motion is off. */
export const dur = (ms = 520) => (still() ? 0 : ms);

/** Options for a `Tween`. */
export const tween = (ms = 520) => ({ duration: dur(ms), easing: cubicOut });
