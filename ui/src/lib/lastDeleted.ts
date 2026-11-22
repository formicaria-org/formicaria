// **The last delete, for every tab** (`ui-audit-2026-11-22.md`, U1). In formicaria's browser each tab
// is a page of its own, so the undo stack (`undo.svelte.ts`) lives and dies with one tab: a note
// deleted in its own tab closed that tab and took its undo along, and the tab the person landed on
// had nothing to offer. The delete is kept here, in the browser's storage, so whichever tab comes
// to the front can say *Deleted “…” · Undo*, and Ctrl+Z there can put it back. What is kept is what
// `restore_note` needs (the vault and the file), and the title to say it: no text of the note.

export interface LastDeleted {
  id: string;
  vault: string;
  file: string;
  title: string;
  at: number;
}

const KEY = 'fm-last-deleted';

export function rememberDeleted(d: Omit<LastDeleted, 'at'>): void {
  try {
    localStorage.setItem(KEY, JSON.stringify({ ...d, at: Date.now() }));
  } catch {
    /* this browser keeps nothing: the tab's own undo still works */
  }
  // This page too: the storage event reaches only the other tabs.
  if (typeof window !== 'undefined') window.dispatchEvent(new CustomEvent(DELETED_EVENT));
}

/** Raised in the page that deleted (`UndoDelete` listens). */
export const DELETED_EVENT = 'fm-deleted';

/** The last delete, if it happened within `withinMs`. */
export function recentDeleted(withinMs: number): LastDeleted | null {
  try {
    const d = JSON.parse(localStorage.getItem(KEY) ?? 'null') as LastDeleted | null;
    if (!d || typeof d.at !== 'number' || !d.vault || !d.file) return null;
    return Date.now() - d.at <= withinMs ? d : null;
  } catch {
    return null;
  }
}

/** Forget it: it was put back, or undone through the tab's own stack. */
export function forgetDeleted(id?: string): void {
  try {
    if (id) {
      const d = JSON.parse(localStorage.getItem(KEY) ?? 'null') as LastDeleted | null;
      if (d && d.id !== id) return;
    }
    localStorage.removeItem(KEY);
  } catch {
    /* nothing kept */
  }
}

export const DELETED_KEY = KEY;
