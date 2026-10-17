// The app's undo — `decisions.md` 2026-10-09, *undo is a session stack of the app's own writes*.
//
// Every undoable write is made through one of the helpers below, which performs the write and
// records how to reverse it, using what the server answers: `delete` hands back the note's file,
// `set_property` the value it replaced. The stack lives for the session (50 entries); a new action
// clears redo. Ctrl+Z / Ctrl+Shift+Z reach it outside text fields and the whiteboard, which keep
// their own undo; on a phone it is the ↶ menu.
//
// **An undo never overwrites a change it did not make.** Before putting a value back it reads the
// note: if the value is no longer the one this action set — the person, a collaborator or another
// device changed it since — it refuses and says so. A deleted note is put back only if no note with
// that id exists again; edited text uses the server's own lost-update guard.
//
// **The text being written is reached first** (owner, 2026-10-17: undo "is not working" on the
// phone). A finished editing session is one step on the stack, but while the note is still being
// written the stack has nothing for it, and a phone has no Ctrl+Z to reach the editor's own history.
// So the editor last written in lends that history here (`holdText`) for as long as it is open: the
// menu and the keys undo its typing, a change at a time, and go on to the stack once it has none.

import {
  capture,
  createNotes,
  deleteNote,
  getNote,
  restoreNote,
  setProperties,
  setProperty,
  updateBody,
} from './ipc';
import type { NoteDetail, NoteDraft } from './types';

export type Undoable = {
  /** What happened, in the person's words: `deleted “Lab meeting”`. */
  label: string;
  undo: () => Promise<void>;
  redo: () => Promise<void>;
};

const LIMIT = 50;

const state = $state({ done: [] as Undoable[], undone: [] as Undoable[], busy: false });

/** An open editor's own history, lent to the menu. `undo`/`redo` answer whether they did anything. */
export type TextHistory = { label: string; undo: () => boolean; redo: () => boolean };

/** Raw, not proxied: `holdText` knows its own history by identity. */
let held = $state.raw<TextHistory | null>(null);
/** How many changes the held history can undo and redo, kept current by the editor. */
const text = $state({ undo: 0, redo: 0 });

/** An editor lends its history while it is written in. Answers how to keep the menu's counts
 *  current, and how to take it back (only if no other editor has taken the place since). */
export function holdText(h: TextHistory) {
  held = h;
  return {
    depth(undo: number, redo: number) {
      if (held !== h) return;
      text.undo = undo;
      text.redo = redo;
    },
    release() {
      if (held !== h) return;
      held = null;
      text.undo = text.redo = 0;
    },
  };
}

const typing = (n: number) => (held && n > 0 ? { label: held.label } : null);

/** Called after an undo or redo lands, so the app can refresh its views and save. Set by `App`. */
let after: () => void | Promise<void> = () => {};
export function onChange(fn: () => void | Promise<void>) {
  after = fn;
}

export function record(u: Undoable) {
  state.done.push(u);
  if (state.done.length > LIMIT) state.done.shift();
  state.undone = [];
}

export const nextUndo = (): { label: string } | null =>
  typing(text.undo) ?? state.done.at(-1) ?? null;
export const nextRedo = (): { label: string } | null =>
  typing(text.redo) ?? state.undone.at(-1) ?? null;
export const busy = () => state.busy;

/** Undo the last action. Resolves to its label, or throws a sentence when it cannot. */
export async function undo(): Promise<string | null> {
  if (held && text.undo > 0 && held.undo()) return held.label;
  const u = state.done.at(-1);
  if (!u || state.busy) return null;
  state.busy = true;
  try {
    try {
      await u.undo();
    } catch (e) {
      // A refused undo is not retried forever: it is dropped, so the one before it is next.
      state.done.pop();
      throw e;
    }
    state.done.pop();
    state.undone.push(u);
    await after();
    return u.label;
  } finally {
    state.busy = false;
  }
}

export async function redo(): Promise<string | null> {
  if (held && text.redo > 0 && held.redo()) return held.label;
  const u = state.undone.at(-1);
  if (!u || state.busy) return null;
  state.busy = true;
  try {
    try {
      await u.redo();
    } catch (e) {
      state.undone.pop();
      throw e;
    }
    state.undone.pop();
    state.done.push(u);
    await after();
    return u.label;
  } finally {
    state.busy = false;
  }
}

/** For tests: start empty. */
export function clear() {
  state.done = [];
  state.undone = [];
  held = null;
  text.undo = text.redo = 0;
}

const CHANGED =
  'it was changed again since, so undoing would overwrite that change — it was left as it is';

/** A property's value in the form `setProperty` reads back (and `set_property` answers). */
function valueOf(n: NoteDetail, key: string): string {
  const v =
    key === 'tags'
      ? n.tags.join(', ')
      : key === 'status' || key === 'title' || key === 'due' || key === 'start'
        ? n[key]
        : n.props?.[key];
  return v == null ? '' : String(v);
}

/** Compare as the server stores it: tags split on commas and trimmed; everything else trimmed. */
function same(key: string, a: string, b: string): boolean {
  if (key === 'tags') {
    const norm = (s: string) =>
      s
        .split(',')
        .map((t) => t.trim())
        .filter(Boolean)
        .join(',');
    return norm(a) === norm(b);
  }
  return a.trim() === b.trim();
}

/** Set one property, undoably. `title` names the note in the label. */
export async function setPropertyUndoable(
  id: string,
  key: string,
  value: string,
  label: string,
): Promise<void> {
  const answer = await setProperty(id, key, value);
  const previous = answer?.previous ?? '';
  if (same(key, previous, value)) return; // nothing changed, nothing to undo
  const put = async (from: string, to: string) => {
    const n = await getNote(id);
    if (!n) throw new Error('that note no longer exists');
    if (!same(key, valueOf(n, key), from)) throw new Error(CHANGED);
    await setProperty(id, key, to);
  };
  record({ label, undo: () => put(value, previous), redo: () => put(previous, value) });
}

/** Delete a note, undoably: undo puts it back exactly as it was. */
export async function deleteUndoable(id: string, title: string): Promise<void> {
  const gone = await deleteNote(id);
  record({
    label: `deleted “${title}”`,
    undo: async () => {
      await restoreNote(gone.vault, gone.file);
    },
    redo: async () => {
      await deleteNote(id);
    },
  });
}

/** Delete several notes as one step — a recipe and what goes in it. */
export async function deleteNotesUndoable(ids: string[], label: string): Promise<void> {
  let gone: { vault: string; file: string }[] = [];
  const all = async () => {
    gone = [];
    for (const id of ids) gone.push(await deleteNote(id));
  };
  await all();
  record({
    label,
    undo: async () => {
      for (const g of gone) await restoreNote(g.vault, g.file);
    },
    redo: all,
  });
}

/** Create a note, undoably: undo removes it (keeping what was typed for redo). */
export async function captureUndoable(body: string, vault: string) {
  const meta = await capture(body, vault);
  let kept: { vault: string; file: string } | null = null;
  record({
    label: 'created a note',
    undo: async () => {
      kept = await deleteNote(meta.id);
    },
    redo: async () => {
      if (kept) await restoreNote(kept.vault, kept.file);
    },
  });
  return meta;
}

/** Create several notes as **one** step: undo removes them all, redo puts them all back. A logged
 *  meal copied from yesterday is several notes and one thing a person did. */
export async function createNotesUndoable(drafts: NoteDraft[], label: string, vault = '') {
  const made = await createNotes(drafts, vault);
  let kept: { vault: string; file: string }[] = [];
  record({
    label,
    undo: async () => {
      kept = [];
      for (const m of made) kept.push(await deleteNote(m.id));
    },
    redo: async () => {
      for (const k of kept) await restoreNote(k.vault, k.file);
    },
  });
  return made;
}

/** Set several properties as one step. */
export async function setPropertiesUndoable(
  id: string,
  props: Record<string, string | number>,
  label: string,
): Promise<void> {
  const { previous } = await setProperties(id, props);
  const back = Object.fromEntries(Object.entries(previous).map(([k, v]) => [k, v ?? '']));
  record({
    label,
    undo: async () => {
      await setProperties(id, back);
    },
    redo: async () => {
      await setProperties(id, props);
    },
  });
}

/** Record a finished editing session on a note's text: undo puts the text back as it was before. */
export function recordTextEdit(id: string, title: string, before: string, after: string) {
  if (before === after) return;
  const put = async (from: string, to: string) => {
    const n = await getNote(id);
    if (!n) throw new Error('that note no longer exists');
    if (n.body !== from) throw new Error(CHANGED);
    await updateBody(id, to, n.version);
  };
  record({
    label: `edits to “${title}”`,
    undo: () => put(after, before),
    redo: () => put(before, after),
  });
}
