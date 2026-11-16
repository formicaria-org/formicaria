// **The note looked at last** (`decisions.md` 2026-11-17): where a selection goes by default when it
// is brought somewhere. Kept on this device; every note page and note window says it is shown.
const KEY = 'fm-last-note';

export function noteShown(id: string, title: string): void {
  try {
    localStorage.setItem(KEY, JSON.stringify({ id, title }));
  } catch {
    /* this device keeps nothing */
  }
}

export function lastNote(): { id: string; title: string } | null {
  try {
    const v = JSON.parse(localStorage.getItem(KEY) ?? 'null') as {
      id?: unknown;
      title?: unknown;
    } | null;
    return v && typeof v.id === 'string'
      ? { id: v.id, title: String(v.title || 'your note') }
      : null;
  } catch {
    return null;
  }
}
