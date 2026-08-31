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


/// **The contributor filter was removed on 2026-08-31**, and `contributors()`/`authorKey()` went
/// with it — the filter was their only caller. The ruling they encoded is not lost: *a contributor
/// is an email, everywhere; the name is only a label* stays in `decisions.md#ui` (2026-07-31),
/// together with the measurement that produced it — one vault, 534 commits as `singhbal-baljinder`
/// and 77 as `Baljinder`, one person. Anything that groups people again must key on the email, not
/// the spelling. `lastEditFor` is untouched: attributing *a note* is a different question from
/// enumerating *people*, and it is still wanted.

