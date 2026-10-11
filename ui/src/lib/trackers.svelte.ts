// Whether the groups that come with the app — Food, Fitness — are shown **on this device**.
//
// Their views are always listed (`ViewInfo.shipped`); showing them is a choice, kept per device
// like the theme that is on (`decisions.md#ui`, 2026-10-11). One place holds it, so the switch in
// Settings and the chrome that draws the groups cannot disagree, and flipping it shows or hides
// them at once.
const KEY = 'fm-trackers';

function read(): boolean {
  try {
    const v = localStorage.getItem(KEY);
    if (v !== null) return v === '1';
  } catch {
    /* no storage: the default */
  }
  // On in `pnpm dev`, so the screens can be worked on; off everywhere else until someone asks.
  return import.meta.env.DEV && import.meta.env.MODE !== 'test';
}

const state = $state({ shown: read() });

export const trackersShown = () => state.shown;

export function showTrackers(on: boolean) {
  state.shown = on;
  try {
    localStorage.setItem(KEY, on ? '1' : '0');
  } catch {
    /* it still holds for this session */
  }
}

/** Read the stored choice again. For tests, which swap storage between cases. */
export function reload() {
  state.shown = read();
}
