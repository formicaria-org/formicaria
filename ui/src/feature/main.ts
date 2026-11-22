// **A feature page** (`decisions.md` 2026-11-17, *one server, many views*): one feature, or one note,
// filling a tab of its own, at `/f/…` on formicaria's server. The same parts as the app (`Pane`, the
// renderers, the note), without the app's bars, panes and windows around them: the browser's tabs are
// the windows. Started as the app is (`lib/start.ts`).
import '../app.css';
import Shell from './Shell.svelte';
import { start } from '../lib/start';

start(Shell);
