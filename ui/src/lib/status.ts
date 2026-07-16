// The click-to-rotate status cycle. Deliberately data-driven: the cycle is the
// set of status values that already exist in the vault (App learns them from
// whatever it fetches), so no status literal is written anywhere in the UI and
// the board stays a group-by-anything view. Typing a brand-new value is still
// the Details editor's job — once a note carries it, it joins the cycle here.

/**
 * The next status after `current`, cycling through `known` and then through
 * unset — so rotating always reaches every value and can always get back to
 * "no status" without a separate clear control.
 *
 * An unknown `current` (a value this browser hasn't seen in a fetch yet) is
 * treated as the start of the cycle, so a click still moves rather than sticking.
 */
export function nextStatus(current: string | null, known: string[]): string | null {
  const cycle: (string | null)[] = [...known, null];
  const at = current === null ? cycle.length - 1 : cycle.indexOf(current);
  if (at < 0) return cycle[0] ?? null;
  return cycle[(at + 1) % cycle.length];
}
