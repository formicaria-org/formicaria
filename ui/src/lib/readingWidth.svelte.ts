// **How wide a note reads** (`decisions.md` 2026-11-21, *reading width*;
// `docs/context/reading-width-research-2026-11-21.md`). The owner, on a laptop: a note read *"super
// short in width"*: the text was capped at 68ch, padding included, with no way to change it.
//
// Conventions, from the research: a readable default inside the 50–75-character band people prefer
// (Baymard; Nanavati & Bias), named steps up to the full width (iA Writer 64/72/80, Bear, Obsidian's
// on/off), and a per-note **Full width** switch (Notion's, and Obsidian's most-wanted plugin). Kept on
// this device, like the theme and the bar's place: a layout choice, not the note's content, so no
// file is rewritten and a phone never inherits a laptop's choice. Shared by every tab of the
// app through `localStorage` and its `storage` event, so a choice made in Settings shows at once in
// a note open in another tab.

export type ReadingWidth = 'narrow' | 'comfortable' | 'wide' | 'full';

/** Characters of text per line for each step; `full` is the pane's own width. */
export const WIDTHS: { value: ReadingWidth; label: string; chars: number | null; why: string }[] = [
  { value: 'narrow', label: 'Narrow', chars: 60, why: 'About 60 characters a line.' },
  {
    value: 'comfortable',
    label: 'Comfortable',
    chars: 75,
    why: 'The default. About 75 characters a line, the width most people find easiest to read.',
  },
  {
    value: 'wide',
    label: 'Wide',
    chars: 100,
    why: 'About 100 characters a line: more on the screen, quicker to skim.',
  },
  { value: 'full', label: 'Full', chars: null, why: 'The text fills the note’s whole width.' },
];

const KEY = 'fm-reading-width';
const FULL_KEY = 'fm-full-width';

function readWidth(): ReadingWidth {
  try {
    const v = localStorage.getItem(KEY);
    return WIDTHS.some((w) => w.value === v) ? (v as ReadingWidth) : 'comfortable';
  } catch {
    return 'comfortable';
  }
}
function readFull(): string[] {
  try {
    const v = JSON.parse(localStorage.getItem(FULL_KEY) ?? '[]');
    return Array.isArray(v) ? v.filter((x) => typeof x === 'string') : [];
  } catch {
    return [];
  }
}

const state = $state({ width: readWidth(), full: readFull() });

if (typeof window !== 'undefined') {
  window.addEventListener('storage', (e) => {
    if (e.key === KEY || e.key === null) state.width = readWidth();
    if (e.key === FULL_KEY || e.key === null) state.full = readFull();
  });
}

export const readingWidth = (): ReadingWidth => state.width;
export function setReadingWidth(w: ReadingWidth): void {
  state.width = w;
  try {
    localStorage.setItem(KEY, w);
  } catch {
    /* this device keeps no choice */
  }
}

/** This note shown at the full width whatever the setting (its Options → Full width). */
export const fullWidth = (id: string): boolean => state.full.includes(id);
export function setFullWidth(id: string, on: boolean): void {
  const rest = state.full.filter((x) => x !== id);
  // The newest 500 kept: a list that only grows would be read on every note opened.
  state.full = on ? [...rest, id].slice(-500) : rest;
  try {
    localStorage.setItem(FULL_KEY, JSON.stringify(state.full));
  } catch {
    /* this device keeps no choice */
  }
}

/** The text measure for a note: `Nch`, or `none` at the full width. */
export function measureFor(id: string | undefined): string {
  if (id && fullWidth(id)) return 'none';
  const chars = WIDTHS.find((w) => w.value === state.width)?.chars ?? null;
  return chars === null ? 'none' : `${chars}ch`;
}

/** Tests: back to the default, nothing remembered. */
export function resetReadingWidth(): void {
  state.width = 'comfortable';
  state.full = [];
}
