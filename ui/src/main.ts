import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';

// Apply the saved theme (or the OS preference) before mount so there's no flash.
//
// **Guarded, like every other storage read in the app.** `localStorage` throws in private mode and
// `matchMedia` is absent in some embedded WebViews — and a throw *here* is the one failure the app's
// own error handling can never reach, because it happens before `App` mounts. On Android that is
// indistinguishable from the bug this file's neighbours exist to fix: a blank window, no console
// (the WebView forwards none), no stdout in logcat, nothing to report.
try {
  const saved = localStorage.getItem('fm-theme');
  const light = globalThis.matchMedia?.('(prefers-color-scheme: light)')?.matches ?? false;
  document.documentElement.dataset.theme = saved ?? (light ? 'light' : 'dark');
} catch {
  document.documentElement.dataset.theme = 'dark';
}

const target = document.getElementById('app');

/// The last resort: words on the screen when the app could not mount at all.
///
/// Not an inline script in `index.html`, which would be the obvious place — the mobile CSP is
/// `script-src 'self' 'wasm-unsafe-eval'` with no `'unsafe-inline'`, so the WebView would refuse to
/// run it and the fallback would itself fail silently. This is plain DOM in the module that already
/// has to load, and it is deliberately dependency-free: no Svelte, no CSS classes, no fonts.
function cannotStart(why: unknown): void {
  const box = target ?? document.body;
  box.textContent = '';
  const p = document.createElement('p');
  p.setAttribute('role', 'alert');
  p.style.cssText = 'padding:2rem;font:1rem system-ui,sans-serif;text-align:center';
  p.textContent = `formicaria could not start: ${String((why as { message?: string })?.message ?? why)}`;
  box.appendChild(p);
}

let app: unknown = null;
try {
  if (!target) throw new Error('the page is missing its #app element');
  app = mount(App, { target });
} catch (e) {
  cannotStart(e);
  throw e;
}

export default app;
