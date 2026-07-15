// User-defined kanban column order, keyed by each column's opaque `value` (the
// grouped property's value). This module is deliberately generic — it never
// inspects what a value means (no status literals), so the board stays a
// group-by-anything view. The order is persisted per group-by in localStorage
// by App.svelte; these are the pure array operations behind that.

/**
 * Arrange `columns` by a saved `order` of values. Values still present in
 * `order` come first, in the saved sequence; any column whose value isn't in
 * `order` keeps its natural relative position, appended after — so a newly
 * appearing column is never hidden, and a saved value that no longer exists is
 * simply skipped. An empty `order` returns the columns unchanged.
 */
export function orderColumns<T extends { value: string }>(columns: T[], order: string[]): T[] {
  if (order.length === 0) return columns;
  const byValue = new Map(columns.map((c) => [c.value, c]));
  const seen = new Set<string>();
  const out: T[] = [];
  for (const v of order) {
    const c = byValue.get(v);
    if (c && !seen.has(v)) {
      out.push(c);
      seen.add(v);
    }
  }
  for (const c of columns) {
    if (!seen.has(c.value)) {
      out.push(c);
      seen.add(c.value);
    }
  }
  return out;
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
