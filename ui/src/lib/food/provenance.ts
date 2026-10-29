// Where a logged value came from, in words (`decisions.md#data`, 2026-10-29). A user logged eggs and
// saw no choline: the egg's table never measures it, and nothing on screen said so — it read as the
// app failing to record it. So an entry says which table its food is from, which of its values were
// borrowed from another table's food, and a total that only some entries carry says why the others
// do not.

/** The tables a food can come from, as a person reads them. */
export const TABLES: Record<string, string> = {
  ciqual: 'Ciqual (France)',
  bls: 'BLS (Germany)',
  swiss: 'the Swiss table',
  crea: 'CREA (Italy)',
  usda: 'USDA (United States)',
};

/** The table an entry's `source` names: `foods@2026.10.4/ciqual:22000` → `ciqual`. */
export function tableOf(source: unknown): string | null {
  if (typeof source !== 'string') return null;
  const m = /\/([a-z]+):/.exec(source);
  return m && TABLES[m[1]] ? m[1] : null;
}

/** An entry's borrowed values, from its `borrowed` property (`choline_ug=USDA: Egg, whole, raw; …`). */
export function borrowedOf(props: Record<string, unknown>): Record<string, string> {
  const raw = props.borrowed;
  if (typeof raw !== 'string' || !raw.trim()) return {};
  const out: Record<string, string> = {};
  for (const part of raw.split(';')) {
    const at = part.indexOf('=');
    if (at > 0) out[part.slice(0, at).trim()] = part.slice(at + 1).trim();
  }
  return out;
}

/** The `borrowed` property for an entry: only the keys it carries, in a stable order. */
export function borrowedProp(borrowed: Record<string, string>, keys: string[]): string {
  return keys
    .filter((k) => borrowed[k])
    .sort()
    .map((k) => `${k}=${borrowed[k]}`)
    .join('; ');
}

/** Keys whose unit changed, as the server's `nutrients::RENAMED`: an entry logged before holds the
 *  old key, read in the new unit. */
const RENAMED: Record<string, [string, number]> = {
  vitamin_b12_ng: ['vitamin_b12_ug', 1000],
  vitamin_d_ng: ['vitamin_d_ug', 1000],
};

/** An entry's whole number under `key`, or under the key it had before its unit changed. */
export function valueOf(props: Record<string, unknown>, key: string): number | null {
  const v = props[key];
  if (typeof v === 'number' && Number.isInteger(v)) return v;
  const old = RENAMED[key];
  if (old) {
    const w = props[old[0]];
    if (typeof w === 'number' && Number.isInteger(w)) return w * old[1];
  }
  return null;
}

/** A nutrient key as a word: `vitamin_b12_ng` → `vitamin B12`, `choline_ug` → `choline`. */
export function nutrientName(key: string): string {
  const words = key.split('_').slice(0, -1);
  return words
    .map((w, i) => (i > 0 && /^[a-z]\d*$/.test(w) ? w.toUpperCase() : w))
    .join(' ')
    .replace(/^vitamin A rae$/, 'vitamin A')
    .replace(/^folate dfe$/, 'folate');
}
