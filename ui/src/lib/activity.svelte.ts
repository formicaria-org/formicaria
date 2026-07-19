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
/** The sentinel `ensure_repo` writes when nobody has said who they are. `git::identity` already
 *  reports it as *absent* rather than as a person — this list has to agree, or a solo vault
 *  grows a phantom collaborator called "formicaria". */
const PLACEHOLDER_EMAIL = 'formicaria@localhost';

/** The people who have touched these notes.
 *
 *  **Deduplicated by email, not by name.** One person is one contributor even when their commits
 *  carry different name spellings — a vault signed `singhbal-baljinder` on one machine and
 *  `Baljinder Singh` on another is still one human, and listing both invites the reasonable
 *  conclusion that someone else is in there. The email is the stable half of a git identity; the
 *  name is what gets shown, taking the most recent spelling.
 *
 *  The placeholder is excluded outright: commits made before an identity was set are
 *  unattributed, which is a different thing from being by someone called "formicaria". */
export function contributors(): string[] {
  const byEmail = new Map<string, string>();
  for (const e of state.events) {
    const email = e.email?.trim().toLowerCase() ?? '';
    if (email === PLACEHOLDER_EMAIL) continue;
    const key = email || e.author;
    // Events arrive newest-first, so the first spelling seen is the most recent one.
    if (!byEmail.has(key)) byEmail.set(key, e.author);
  }
  return [...byEmail.values()].sort((a, b) => a.localeCompare(b));
}
