// Where the manual is, and which page of it is open — `decisions.md` 2026-10-18, *the full manual on
// every device, and a link beside each feature to its section*.
//
// One reader on every OS (`ManualPanel.svelte`); only where the book is served differs. On a
// computer it is built into the program at `/manual/`. On a phone the shell serves its own copy on
// the `fmmanual:` scheme — a different origin from the app's, on purpose.

import { blobBase } from './blobBase';
import { isPhone } from './platform';

/** The base the book is served from, with a trailing slash. */
export function manualBase(): string {
  if (!isPhone()) return '/manual/';
  const internals = (
    window as unknown as {
      __TAURI_INTERNALS__?: { convertFileSrc?: (path: string, protocol: string) => string };
    }
  ).__TAURI_INTERNALS__;
  return blobBase(internals?.convertFileSrc, 'fmmanual');
}

/** A page of the book (`user/appearance.html`), at a section if one is named
 *  (`starting-from-a-suggestion`, the id mdBook gives that heading). */
export function manualUrl(page = 'index.html', section?: string): string {
  return `${manualBase()}${page}${section ? `#${section}` : ''}`;
}

const state = $state({ url: null as string | null });

/** The page open in the reader, or null. */
export const openPage = () => state.url;

/** Open the reader at a page and section. */
export function openManual(page?: string, section?: string) {
  state.url = manualUrl(page, section);
}

export function closeManual() {
  state.url = null;
}
