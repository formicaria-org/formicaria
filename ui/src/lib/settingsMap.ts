/// How Settings is divided, and what is on screen for a given category and search.
///
/// Pure, so the rules have tests and the panel only applies them (`decisions.md`, 2026-10-11,
/// *Settings shows one category at a time*). The sections themselves say which category they are
/// in (`data-cat` in the markup); this is the list of categories and the one question "is this
/// section shown now?".

export const CATEGORIES = [
  { id: 'look', label: 'Look' },
  { id: 'keyboard', label: 'Keyboard' },
  // **The app's clock** (`decisions.md#data`, 2026-10-24): on every device, the phone included.
  { id: 'clock', label: 'Clock' },
  // **A vault's settings have a category of their own** (`decisions.md#ui`, 2026-10-15): where it
  // lives, where it is sent, its snapshot, which attachments travel, making and removing vaults.
  { id: 'vaults', label: 'Vaults & backup' },
  { id: 'notes', label: 'Food & reminders' },
  { id: 'assistant', label: 'Assistant' },
  { id: 'connections', label: 'Connections' },
  { id: 'about', label: 'About & updates' },
] as const;

export type Cat = (typeof CATEGORIES)[number]['id'];
/// Where a section lives: a category, or the window of actions the ＋ opens.
export type Place = Cat | 'actions';

export function labelOf(cat: Cat): string {
  return CATEGORIES.find((c) => c.id === cat)?.label ?? cat;
}

export function isCat(x: unknown): x is Cat {
  return CATEGORIES.some((c) => c.id === x);
}

/// Does `text` match what was typed? Every word typed must be in it, in any order, whatever the
/// case: "backup itself" finds *Backs up by itself*.
export function matches(text: string, query: string): boolean {
  const hay = text.toLowerCase();
  return query
    .toLowerCase()
    .split(/\s+/)
    .filter(Boolean)
    .every((w) => hay.includes(w));
}

/// Is a section on screen?
///
/// - In the actions window: the actions, and nothing else.
/// - In Settings: never the actions. With something typed in the search box, every section whose
///   words match, from any category; otherwise the chosen category's.
/// - A label that only heads a group of sections (`heading`) is not a search result by itself.
export function shown(a: {
  actions: boolean;
  cat: Cat;
  query: string;
  place: Place;
  text: string;
  heading?: boolean;
}): boolean {
  if (a.actions) return a.place === 'actions';
  if (a.place === 'actions') return false;
  const q = a.query.trim();
  if (q) return !a.heading && matches(a.text, q);
  return a.place === a.cat;
}
