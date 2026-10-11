/// The smallest replacement that turns `was` into `now`: what they share at the start and at the
/// end is left alone. An editor applying an outside change this way keeps its cursor, its scroll
/// and its undo history sensible, where replacing the whole text would reset all three.
export function smallestChange(
  was: string,
  now: string,
): { from: number; to: number; insert: string } | null {
  if (was === now) return null;
  let start = 0;
  const max = Math.min(was.length, now.length);
  while (start < max && was.charCodeAt(start) === now.charCodeAt(start)) start++;
  let endWas = was.length;
  let endNow = now.length;
  while (
    endWas > start &&
    endNow > start &&
    was.charCodeAt(endWas - 1) === now.charCodeAt(endNow - 1)
  ) {
    endWas--;
    endNow--;
  }
  return { from: start, to: endWas, insert: now.slice(start, endNow) };
}
