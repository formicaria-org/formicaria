// The workspace's git activity, as one reactive singleton — so the "edited by" labels reach
// every renderer without prop-drilling through Pane. App fetches `activity` and calls `setActivity`
// on the same cadence it refreshes feeds; renderers `import { lastEditFor }` and read it. This is
// a Svelte-5 runes-in-module store (`.svelte.ts`): mutating the exported `$state` re-renders every
// reader. Nothing is stored on disk — git is the source of truth; this is just the last fetch.

import type { EditEvent } from './types';

const state = $state<{ events: EditEvent[]; byId: Record<string, EditEvent> }>({
  events: [],
  byId: {},
});

/** Replace the activity with the latest fetch. `events` is newest-first, so the first time a note
 *  id appears is its last edit — exactly what `byId` keeps. */
export function setActivity(events: EditEvent[]): void {
  const byId: Record<string, EditEvent> = {};
  for (const e of events) byId[e.id] ??= e;
  state.events = events;
  state.byId = byId;
}

/** This note's most-recent edit, or undefined when git knows nothing about it (no history, or a
 *  single-user vault with no commits yet). */
export function lastEditFor(id: string): EditEvent | undefined {
  return state.byId[id];
}

/** The activity feed, newest-first — what the Activity pane renders. Reactive. */
export function activityEvents(): EditEvent[] {
  return state.events;
}

/** Distinct contributors seen in the current activity, sorted — the contributor filter chips. */
export function contributors(): string[] {
  return [...new Set(state.events.map((e) => e.author))].sort((a, b) => a.localeCompare(b));
}
