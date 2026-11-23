// **A red dot on Collaboration** (2026-11-23; the owner: *"a small red dot on the collaboration view
// in the list of views … as non invasive as possible"*). It says the assistant has proposals you
// have not looked at: open (not accepted, declined or closed) and never shown (`shown`, which the
// review screen already records). Nothing new is stored, and nothing is sent outside the app.
import * as ipc from './ipc';
import type { ObjectMeta } from './types';

const set = (v: unknown) =>
  v !== undefined && v !== null && v !== '' && v !== false && v !== 'false';

/** Whether a proposal is open and has never been shown to anyone. */
export function waitingProposal(p: ObjectMeta): boolean {
  const pr = p.props ?? {};
  return !set(pr.declined) && !set(pr.accepted) && !set(pr.closed) && !set(pr.shown);
}

let n = $state(0);
let started = false;

/** How many proposals wait to be looked at; read once, then kept current by the change stream. */
export function proposalsWaiting(): number {
  if (!started && typeof window !== 'undefined') {
    started = true;
    const read = () =>
      void ipc
        .proposals()
        .then((ps) => (n = ps.filter(waitingProposal).length))
        .catch(() => {});
    read();
    ipc.listenChanges(read);
  }
  return n;
}

/** For tests: forget the count and read again on the next ask. */
export function resetWaiting(): void {
  n = 0;
  started = false;
}

/** The dot's words, for its title and for screen readers. */
export const waitingLabel = (count: number) =>
  `${count} ${count === 1 ? 'proposal' : 'proposals'} to look at`;
