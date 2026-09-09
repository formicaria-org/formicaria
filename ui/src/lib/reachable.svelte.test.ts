// Telling "the app has stopped" apart from "the app said no".
//
// Issue #2's reporter saw `TypeError: Load failed` in a note pane, reloaded, and got nowhere —
// because the server was not running and nothing on screen said so. `fetch` already distinguishes
// the two cases (it rejects for an unreachable server and resolves for a refusal); what was
// missing was anywhere to put that distinction. These tests pin the policy, which is a judgement
// about false alarms rather than a mechanism.
import { beforeEach, describe, expect, it } from 'vitest';
import { missed, reached, resetReachable, unreachable } from './reachable.svelte';

beforeEach(() => resetReachable());

describe('has the app on this computer stopped', () => {
  it('says nothing at rest', () => {
    expect(unreachable()).toBe(false);
  });

  it('does not cry wolf on a single failure', () => {
    // One failed request is ordinary — a command cancelled by a navigation, a socket used after
    // the server's 30s read timeout, a laptop waking up. "formicaria has stopped" is the most
    // alarming sentence this app can say; it must not be said on one miss.
    missed();
    expect(unreachable()).toBe(false);
  });

  it('says so once the failures are consistent', () => {
    missed();
    missed();
    expect(unreachable()).toBe(true);
  });

  it('any answer at all clears it — including a refusal', () => {
    // The question is "is it running", not "did it agree". An error status is proof of life, and
    // treating a refusal as absence would announce a dead app every time a command was declined.
    missed();
    missed();
    expect(unreachable()).toBe(true);
    reached();
    expect(unreachable()).toBe(false);
  });

  it('a success between failures resets the run, so intermittent loss is not absence', () => {
    missed();
    reached();
    missed();
    expect(unreachable()).toBe(false);
  });
});
