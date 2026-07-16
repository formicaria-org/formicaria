// User-defined kanban order — of the columns (keyed by each column's opaque
// `value`, the grouped property's value) and of the cards within a column (keyed
// by note id). This module is deliberately generic — it never inspects what a
// value means (no status literals), so the board stays a group-by-anything view.
// Both orders are persisted per group-by in localStorage by App.svelte; these
// are the pure array operations behind that.

/**
 * Arrange `items` by a saved `order` of keys. Items whose key is in `order` come
 * first, in the saved sequence; anything else keeps its natural relative
 * position, appended after — so a newly appearing item is never hidden, and a
 * saved key that no longer exists is simply skipped. An empty `order` returns
 * `items` unchanged.
 */
export function arrange<T>(items: T[], order: string[], key: (item: T) => string): T[] {
  if (order.length === 0) return items;
  const byKey = new Map(items.map((i) => [key(i), i]));
  const seen = new Set<string>();
  const out: T[] = [];
  for (const k of order) {
    const item = byKey.get(k);
    if (item !== undefined && !seen.has(k)) {
      out.push(item);
      seen.add(k);
    }
  }
  for (const item of items) {
    if (!seen.has(key(item))) {
      out.push(item);
      seen.add(key(item));
    }
  }
  return out;
}

/** Arrange `columns` by a saved `order` of their values. See {@link arrange}. */
export function orderColumns<T extends { value: string }>(columns: T[], order: string[]): T[] {
  return arrange(columns, order, (c) => c.value);
}

/**
 * Return a new full ordering with `fromValue` repositioned relative to
 * `toValue` (inserted before it when `before`, else after). No-op when the two
 * are equal or `toValue` is absent. Operates on the current full value list, so
 * the first drag produces a complete, stable permutation to persist.
 */
export function moveValue(
  values: string[],
  fromValue: string,
  toValue: string,
  before: boolean,
): string[] {
  if (fromValue === toValue) return values;
  const without = values.filter((v) => v !== fromValue);
  const idx = without.indexOf(toValue);
  if (idx < 0) return values;
  without.splice(before ? idx : idx + 1, 0, fromValue);
  return without;
}

/**
 * Return a new ordering with `id` placed immediately before `beforeId`, or
 * appended when `beforeId` is null (dropped past the last card) or unknown.
 * `id` need not already be in `values` — a card dragged in from another column
 * is simply inserted, which is what makes one function serve both cases.
 *
 * Unlike `moveValue` this takes the *whole* column's current id list, so the
 * first drop persists a complete, stable order rather than a partial one.
 */
export function placeValue(values: string[], id: string, beforeId: string | null): string[] {
  // "Before itself" is no move at all. Unreachable from a drag (the board skips
  // the dragged card when picking a target), but without this the filter below
  // would drop the card and the append fallback would teleport it to the end.
  if (beforeId === id) return values;
  const without = values.filter((v) => v !== id);
  const idx = beforeId === null ? -1 : without.indexOf(beforeId);
  if (idx < 0) return [...without, id];
  without.splice(idx, 0, id);
  return without;
}
