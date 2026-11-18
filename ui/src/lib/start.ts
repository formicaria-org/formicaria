// **How each of formicaria's pages starts** (`lean-plan-2026-11-18.md` L4, stage A): the app
// (`main.ts`, the phone) and a feature page (`feature/main.ts`, the computer) had each their own copy of
// the theme's first paint, and only the app could say it failed to start. One copy, both entries.
import { mount, type Component } from 'svelte';

/// The saved theme (or the system's), set before anything is drawn so there is no flash.
///
/// **Guarded**: `localStorage` throws in private mode and `matchMedia` is absent in some embedded
/// WebViews, and a throw here happens before the app mounts, where nothing else can report it.
export function applyTheme(): void {
  try {
    const saved = localStorage.getItem('fm-theme');
    const light = globalThis.matchMedia?.('(prefers-color-scheme: light)')?.matches ?? false;
    document.documentElement.dataset.theme = saved ?? (light ? 'light' : 'dark');
  } catch {
    document.documentElement.dataset.theme = 'dark';
  }
}

/// The last resort: words on the screen when the page could not mount at all.
///
/// Not an inline script in the HTML: the phone's CSP (`script-src 'self' 'wasm-unsafe-eval'`, no
/// `'unsafe-inline'`) would refuse it and the fallback would itself fail silently. Plain DOM,
/// deliberately dependency-free: no Svelte, no CSS classes, no fonts.
export function cannotStart(target: HTMLElement | null, why: unknown): void {
  const box = target ?? document.body;
  box.textContent = '';
  const p = document.createElement('p');
  p.setAttribute('role', 'alert');
  p.style.cssText = 'padding:2rem;font:1rem system-ui,sans-serif;text-align:center';
  p.textContent = `formicaria could not start: ${String((why as { message?: string })?.message ?? why)}`;
  box.appendChild(p);
}

/// Mount `component` into `#app`, or say why it could not be (and rethrow, for the console).
export function start(component: Component): unknown {
  applyTheme();
  const target = document.getElementById('app');
  try {
    if (!target) throw new Error('the page is missing its #app element');
    return mount(component, { target });
  } catch (e) {
    cannotStart(target, e);
    throw e;
  }
}

/// The other theme, worn and kept on this device: the one toggle both pages use (the app's copy did
/// not guard the write, so storage refusing threw out of the click).
export function toggledTheme(theme: string): 'dark' | 'light' {
  const next = theme === 'dark' ? 'light' : 'dark';
  document.documentElement.dataset.theme = next;
  try {
    localStorage.setItem('fm-theme', next);
  } catch {
    /* this device keeps no choice: worn for now */
  }
  return next;
}
