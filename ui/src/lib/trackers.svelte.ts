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
  return true;
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

// **A recipe to open the food sheet on**, asked for from somewhere else — a recipe's own note
// (2026-10-18). Held here until the food view is on screen to take it, then taken once.
const asked = $state({ ref: null as string | null });
/** Ask the food view to open its sheet on `recipe/<word>` when it next shows. */
export function askToOpen(ref: string) {
  asked.ref = ref;
}
/** What was asked for, once: the asker's request is spent by reading it. */
export function takeAsked(): string | null {
  const ref = asked.ref;
  asked.ref = null;
  return ref;
}
/** Whether something is waiting — reactive, so the food view can notice it arrive. */
export const isAsked = () => asked.ref !== null;
