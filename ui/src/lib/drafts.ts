// What was typed into a form and not yet saved, kept on this device so it is still there after
// leaving the form, changing view, or closing the app (`decisions.md#ui`, 2026-10-14, *a form's
// unsaved work is kept on the device*).
//
// The owner lost a recipe half built. A recipe is now a note from its first word; these are the
// forms that are one screen and write once, when their button is pressed: until then their
// fields are kept here, restored when the form is next opened, and dropped when it is saved or
// the person starts afresh. Per device, like a view's last tab: a draft is not a note, is in no
// backup, and is never sent anywhere.
const PREFIX = 'fm-draft:';

/** The draft kept under `name`, or nothing. */
export function draftOf<T>(name: string): T | null {
  try {
    const raw = localStorage.getItem(PREFIX + name);
    return raw ? (JSON.parse(raw) as T) : null;
  } catch {
    return null;
  }
}

/** Keep `value` as the draft under `name`. A device that cannot store it simply does not. */
export function keepDraft(name: string, value: unknown): void {
  try {
    localStorage.setItem(PREFIX + name, JSON.stringify(value));
  } catch {
    // Storage is full or refused: the form still works, it is just not kept.
  }
}

export function dropDraft(name: string): void {
  try {
    localStorage.removeItem(PREFIX + name);
  } catch {
    // Nothing was kept.
  }
}

/** A keeper for one form: given what the form holds now, it keeps that as the draft — or drops
 *  the draft when it is back to `initial`, what the form opened on before any draft was put
 *  back. So opening something to look at it, and changing nothing, leaves no draft behind. */
export function drafting(name: string, initial: unknown): (now: unknown) => void {
  const base = JSON.stringify(initial);
  return (now) => (JSON.stringify(now) === base ? dropDraft(name) : keepDraft(name, now));
}
