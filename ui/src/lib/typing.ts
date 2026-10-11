/// Is the keyboard being used to write, in this element?
///
/// One answer for the whole app, because two places need it and they had drifted: the shortcut
/// handler asked only about text boxes, so in the editor that renders as you write — which is not
/// a text box — a bare `c` opened a new note in the middle of a word (the owner, 2026-10-11).
/// Anything a person types into counts: a field, a text box, a list to choose from, and any
/// element the browser lets them edit.
export function isTyping(el: EventTarget | Element | null | undefined): boolean {
  if (!(el instanceof HTMLElement)) return false;
  const tag = el.tagName;
  if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return true;
  // `isContentEditable` is the truth where there is a layout engine; the attribute is what a test
  // environment without one can see.
  return el.isContentEditable || !!el.closest('[contenteditable="true"], [contenteditable=""]');
}
