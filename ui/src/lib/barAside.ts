/// **Typing with the on-screen keyboard up, anywhere on the page** (2026-11-24).
///
/// On a phone the keyboard takes half the screen, and the bottom bar (☰, the tabs, +) sat between
/// what was being typed and the keys. It already stepped aside for a note's own text; the journal on
/// Home, a search field in a view and the logging sheet kept it, so the line being typed got a
/// sliver of screen (the owner, 2026-11-23: *"remember that the keyboard appears and take up a big
/// portion of the screen!"*).
///
/// So the rule is one rule for the whole page: while a text field has the focus **and** the
/// keyboard is up, `<html>` carries `data-typing`, and the bar steps aside (`App.svelte`). Except
/// for a field inside the bar itself — its search raises the keyboard too, and must not hide the
/// bar it lives in. Putting the keyboard away (Android's Back, the keyboard's own button) brings the
/// bar back even though the field keeps the focus: the keyboard, not the focus, is what took the
/// room.
///
/// "The keyboard is up" is `--kb` where the Android shell writes it (`lib/keyboard.ts`), and
/// otherwise a visual viewport clearly shorter than the window, on a touch screen — a phone's
/// browser shrinks its own viewport instead. A desktop has neither, so this never fires there.
import { KEYBOARD_EVENT, keyboardInset } from './keyboard';

export const TYPING_ATTR = 'data-typing';

/// A viewport this much shorter than the window is a keyboard, not a browser's address bar
/// sliding away (that is ~56–80 CSS px).
const SHRUNK_PX = 150;

const NOT_TYPED = new Set([
  'button',
  'checkbox',
  'color',
  'file',
  'hidden',
  'image',
  'radio',
  'range',
  'reset',
  'submit',
]);

/// A field the keyboard types into: a text-like input, a text area, or editable text (the editor).
/// Narrower than `isTyping` (`lib/typing.ts`), which also counts a list to choose from and a
/// checkbox: those keep shortcuts away, but raise no keyboard.
export function typesText(el: Element | null): boolean {
  if (!el || !(el instanceof HTMLElement)) return false;
  if (el instanceof HTMLTextAreaElement) return !el.readOnly && !el.disabled;
  if (el instanceof HTMLInputElement)
    return !NOT_TYPED.has(el.type) && !el.readOnly && !el.disabled;
  // `isContentEditable` is what a browser answers; jsdom leaves it undefined, so ask the attribute.
  return (
    el.isContentEditable === true ||
    el.closest('[contenteditable=""],[contenteditable="true"]') !== null
  );
}

export function keyboardUp(): boolean {
  if (keyboardInset() > 0) return true;
  const vv = window.visualViewport;
  if (!vv || !window.matchMedia?.('(pointer: coarse)').matches) return false;
  return window.innerHeight - vv.height > SHRUNK_PX;
}

/// Keep `data-typing` on `<html>` true to the page. `keeps(el)` names fields that must not hide
/// the bar (the bar's own). Returns the function that stops watching.
export function watchTyping(keeps: (el: Element) => boolean = () => false): () => void {
  const root = document.documentElement;
  const update = () => {
    const el = document.activeElement;
    const on = typesText(el) && !(el && keeps(el)) && keyboardUp();
    if (root.hasAttribute(TYPING_ATTR) !== on) root.toggleAttribute(TYPING_ATTR, on);
  };
  // Moving from one field to the next blurs before it focuses; asking a moment later keeps the bar
  // from flickering back for that instant.
  let pending = 0;
  const soon = () => {
    clearTimeout(pending);
    pending = window.setTimeout(update, 0);
  };
  document.addEventListener('focusin', update);
  document.addEventListener('focusout', soon);
  window.addEventListener(KEYBOARD_EVENT, update);
  window.visualViewport?.addEventListener('resize', update);
  update();
  return () => {
    clearTimeout(pending);
    document.removeEventListener('focusin', update);
    document.removeEventListener('focusout', soon);
    window.removeEventListener(KEYBOARD_EVENT, update);
    window.visualViewport?.removeEventListener('resize', update);
    root.removeAttribute(TYPING_ATTR);
  };
}
