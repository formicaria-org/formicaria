/// **How much of the screen the on-screen keyboard covers, in CSS pixels.**
///
/// The Android shell draws edge-to-edge and targets SDK 36, where the system no longer resizes the
/// window for the keyboard: the WebView stays full height and the keyboard is drawn *over* it, so
/// `100dvh`, `visualViewport` and the browser's own "scroll the focused field into view" all believe
/// the whole screen is visible. The editor's last lines — the ones being typed — sat under the
/// keyboard (the owner, 2026-10-08).
///
/// So the shell reports the overlap the way it already reports the safe-area insets: `MainActivity`
/// writes `--kb` on `:root` (and `data-keyboard` while it is non-zero) and fires `KEYBOARD_EVENT`.
/// `app.css` takes `--kb` out of `--app-h`, which is what the shell and every overlay are sized by.
/// Everywhere else `--kb` is never written and stays `0px`: a desktop has no on-screen keyboard, and
/// a browser or WKWebView resizes its own viewport.
export const KEYBOARD_EVENT = 'fm-keyboard';

export function keyboardInset(): number {
  const raw = document.documentElement.style.getPropertyValue('--kb');
  const n = parseFloat(raw);
  return Number.isFinite(n) && n > 0 ? n : 0;
}
