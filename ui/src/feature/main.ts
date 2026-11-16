// **A feature page** (`decisions.md` 2026-11-17, *one server, many views*): one feature, or one note,
// filling a tab of its own, at `/f/…` on formicaria's server. The same parts as the app (`Pane`, the
// renderers, the note), without the app's bars, panes and windows around them: the browser's tabs are
// the windows.
import '../app.css';
import { mount } from 'svelte';
import Feature from './Feature.svelte';

try {
  const saved = localStorage.getItem('fm-theme');
  const light = globalThis.matchMedia?.('(prefers-color-scheme: light)')?.matches ?? false;
  document.documentElement.dataset.theme = saved ?? (light ? 'light' : 'dark');
} catch {
  document.documentElement.dataset.theme = 'dark';
}

const target = document.getElementById('app');
if (target) mount(Feature, { target });
