// **The roller** (owner, 2026-11-17: *"something like the slot machine: a rolling thing that has
// different options as the user rolls it … to avoid a horizontal tiling"*; *"the roller or the arc of
// open tabs/notes"*): the phone's open notes and pages on an arc along the bottom bar. The one in
// front sits in the middle, largest; its neighbours curve away and shrink. Rolling with the thumb
// moves the arc; letting go settles on the nearest and opens it; a wheel notch moves one.
//
// The geometry and the settling are here, pure, so they are tested without a screen. **No gesture
// memory** (`known-issues.md`, *A gesture control must not remember what you did last*): where a roll
// lands depends only on where it started and how far it went, and one notch is one item either way.

/** How far a finger moves (px) to roll one item. */
export const STEP = 76;
/** How many items show on each side of the middle. */
export const SIDE = 2;

/** Where an item sits on the arc, `off` items from the middle (fractional while rolling). */
export function place(off: number): {
  x: number;
  y: number;
  rotate: number;
  scale: number;
  opacity: number;
} {
  const a = Math.min(Math.abs(off), SIDE + 1);
  return {
    x: off * STEP,
    // Down the arc as it leaves the middle: a parabola, so the middle is its top.
    y: a * a * 7,
    rotate: Math.max(-40, Math.min(40, off * 12)),
    scale: 1 - Math.min(a, SIDE + 1) * 0.14,
    opacity: a > SIDE + 0.5 ? 0 : 1 - a * 0.28,
  };
}

const clamp = (i: number, n: number) => Math.max(0, Math.min(n - 1, i));

/** **Which item a roll lands on**: from `from`, after the finger moved `dx` (right is back, as a
 *  wheel turned by its rim), the nearest item, kept within the `n` there are. */
export function settle(from: number, dx: number, n: number): number {
  if (n <= 0) return 0;
  return clamp(Math.round(from - dx / STEP), n);
}

/** The middle while rolling: fractional, so the arc follows the finger. */
export function rolling(from: number, dx: number, n: number): number {
  if (n <= 0) return 0;
  return Math.max(-0.4, Math.min(n - 0.6, from - dx / STEP));
}

/** **One wheel notch, one item**, either way, whatever came before. */
export function notch(from: number, deltaY: number, n: number): number {
  if (n <= 0 || deltaY === 0) return clamp(from, n);
  return clamp(from + Math.sign(deltaY), n);
}
