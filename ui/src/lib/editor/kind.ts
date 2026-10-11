/// Which editor a note opens in on this device: the plain text box, or the one that renders as
/// you write. Plain unless chosen otherwise (`decisions.md`, 2026-10-11): the live editor is new,
/// and the plain one is what has been proven on a phone.
export type EditorKind = 'plain' | 'live';
export const EDITOR_KEY = 'fm-editor';

export function editorKind(): EditorKind {
  try {
    return localStorage.getItem(EDITOR_KEY) === 'live' ? 'live' : 'plain';
  } catch {
    return 'plain';
  }
}

export function setEditorKind(kind: EditorKind): void {
  try {
    localStorage.setItem(EDITOR_KEY, kind);
  } catch {
    /* a browser with no storage keeps the plain editor */
  }
}

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
