// **The app** (the phone; and the computer's `/`): its start is shared with the feature pages
// (`lib/start.ts`): the theme before the first paint, and words on the screen if mounting fails.
import './app.css';
import App from './App.svelte';
import { start } from './lib/start';

export default start(App);
