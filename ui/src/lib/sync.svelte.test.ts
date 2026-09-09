// The sync sequence, driven through every branch with fake git operations.
//
// These are the states that matter and that a real remote will not hold still for: a push
// rejected because someone else pushed, a merge that genuinely disagrees, a push that fails
// for a reason pulling cannot fix. Each has a rule attached, and the rules are the reason
// this module exists rather than a `commit().then(push)`.

import { describe, expect, it, vi } from 'vitest';
import {
  busyLabel,
  clearSync,
  plainError,
  pullVault,
  syncFor,
  syncVault,
  syncingPhase,
  type SyncOps,
} from './sync.svelte';

/** A fake git. `pushes` counts attempts, which is how "exactly one retry" is asserted. */
function ops(over: Partial<SyncOps> = {}) {
  const calls = { commit: 0, push: 0, pull: 0 };
  const base: SyncOps = {
    // Default: this vault has a remote, so a failure stays a failure. The no-remote case is its own
    // test below.
    hasRemote: async () => true,
    commit: async () => {
      calls.commit++;
      return { committed: true, conflicts: [] };
    },
    push: async () => void calls.push++,
    pull: async () => {
      calls.pull++;
      return { merged: 0, conflicts: [], kept: [] };
    },
  };
  return { calls, ops: { ...base, ...over } as SyncOps };
}

/** A push that is rejected the first N times, then succeeds. */
function rejectingPush(times: number, calls: { push: number }) {
  return async () => {
    calls.push++;
    if (calls.push <= times) throw new Error("the remote has changes you don't have");
  };
}

describe('syncVault', () => {
  it('gets their changes and sends yours, in that order, every time', async () => {
    // **Backing up is both halves since 2026-09-09.** It used to pull only when a push came back
    // rejected, which left the *decision* with the user: notice a chip, press "Get their changes",
    // then press Back up. That decision has one right answer, so the program makes it.
    // After the commit, never before it: git will not merge over uncommitted edits, and the
    // five-second debounce means the tree is dirty exactly when someone has just been typing.
    clearSync('v');
    const { calls, ops: o } = ops();

    expect(await syncVault('v', 'msg', undefined, o)).toBe('synced');

    expect(calls).toEqual({ commit: 1, push: 1, pull: 1 });
    expect(syncFor('v').phase).toBe('synced');
  });

  // The safety net behind the up-front pull: the remote can still move between that pull and this
  // push, and when it does the retry is what saves the backup.
  it('heals a remote that moved after the pull: rejected push -> pull -> push once more', async () => {
    clearSync('v');
    const { calls, ops: o } = ops();
    o.push = rejectingPush(1, calls);
    o.pull = async () => {
      calls.pull++;
      return { merged: 3, conflicts: [], kept: [] };
    };
    const onChanged = vi.fn();

    expect(await syncVault('v', 'msg', onChanged, o)).toBe('synced');

    expect(calls.push).toBe(2);
    // Twice: once up front as part of backing up, once more to heal the rejection.
    expect(calls.pull).toBe(2);
    expect(onChanged).toHaveBeenCalledTimes(2);
    expect(syncFor('v').merged).toBe(3);
  });

  // The rule that makes this safe. Conflict markers are not content.
  //
  // **Stronger than it was.** The pull now happens before the push, so a conflict is found before
  // anything is sent at all — the count below is zero where it used to be one. The old sequence
  // could only refuse the *second* push, after the first had already gone out.
  it('never pushes at all when the pull conflicted', async () => {
    clearSync('v');
    const { calls, ops: o } = ops();
    o.push = rejectingPush(1, calls);
    o.pull = async () => {
      calls.pull++;
      return { merged: 0, conflicts: ['01AAA.md', '01BBB.md'], kept: [] };
    };

    expect(await syncVault('v', 'msg', undefined, o)).toBe('conflicts');

    // No push happens at all: the conflict is found before anything is sent.
    expect(calls.push).toBe(0);
    expect(syncFor('v').conflicts).toEqual(['01AAA.md', '01BBB.md']);
  });

  // The failure mode the plan warns about: a push that keeps being rejected must stop,
  // not spin. Pulling brought nothing down, so the remote had not moved and retrying
  // would fail identically.
  it('stops instead of looping when the push fails for its own reasons', async () => {
    clearSync('v');
    const { calls, ops: o } = ops();
    o.push = async () => {
      calls.push++;
      throw new Error('could not read Username: terminal prompts disabled');
    };

    expect(await syncVault('v', 'msg', undefined, o)).toBe('failed');

    // No retry: the pull brought nothing, so a retry would fail identically.
    expect(calls.push).toBe(1);
    expect(syncFor('v').error).toContain('terminal prompts disabled');
  });

  // Even a second push can fail (the remote moved *again*). One retry means one.
  it('gives up after exactly one retry', async () => {
    clearSync('v');
    const { calls, ops: o } = ops();
    o.push = rejectingPush(99, calls);
    o.pull = async () => {
      calls.pull++;
      return { merged: 1, conflicts: [], kept: [] };
    };

    expect(await syncVault('v', 'msg', undefined, o)).toBe('failed');

    // Tried twice, then stopped. One retry means one, however many times the remote moves.
    expect(calls.push).toBe(2);
    // Twice: the up-front pull that backing up now always does, and the one the rejection asks
    // for. The retry count is about *pushes* — that is the thing with an audience.
    expect(calls.pull).toBe(2);
  });

  // **The commit now succeeds while the merge is still unfinished, and the sync must still stop.**
  // Regression guard for the proxy this file used to rely on: `!committed && conflicts.length`.
  // Since 2026-09-07 `commit_all` commits everything except the conflicted paths, so `committed`
  // is `true` in exactly the case that must not proceed — `pull` and `push` both refuse over an
  // unfinished merge, and the user would get git's wording instead of a named phase.
  it('stops at conflicts even when the commit succeeded', async () => {
    clearSync('v');
    const { calls, ops: o } = ops();
    o.commit = async () => {
      calls.commit++;
      return { committed: true, conflicts: ['notes/01AAAAAAAAAAAAAAAAAAAAAAAA.md'] };
    };

    expect(await syncVault('v', 'msg', undefined, o)).toBe('conflicts');

    expect(calls.pull).toBe(0);
    expect(calls.push).toBe(0);
    expect(syncFor('v').conflicts).toHaveLength(1);
  });

  // A vault mid-merge refuses to commit. Pushing past that would publish a half-merge.
  it('does not push when the commit refused', async () => {
    clearSync('v');
    const { calls, ops: o } = ops();
    o.commit = async () => {
      calls.commit++;
      throw new Error('there is already a merge to finish here');
    };

    expect(await syncVault('v', 'msg', undefined, o)).toBe('failed');

    expect(calls.push).toBe(0);
    expect(syncFor('v').error).toContain('merge to finish');
  });

  // The silent freeze. `commit_all` does not *throw* while the vault is mid-merge — it
  // returns "committed nothing", which is also what a clean tree returns. Read as success,
  // every save after the conflict is written to disk and never committed, indefinitely,
  // while the UI reports `synced`. The conflicted notes are the signal that tells them apart.
  it('stops and names the notes when a conflict is silently blocking every commit', async () => {
    clearSync('v');
    const { calls, ops: o } = ops();
    o.commit = async () => {
      calls.commit++;
      return { committed: false, conflicts: ['notes/01ARZ3.md'] };
    };

    expect(await syncVault('v', 'msg', undefined, o)).toBe('conflicts');

    // Never publish over a vault that has stopped recording.
    expect(calls.push).toBe(0);
    expect(syncFor('v').conflicts).toEqual(['notes/01ARZ3.md']);
  });

  // ...but "committed nothing" on a clean tree is the ordinary case and must not stop it.
  it('carries on when there was simply nothing to commit', async () => {
    clearSync('v');
    const { calls, ops: o } = ops();
    o.commit = async () => {
      calls.commit++;
      return { committed: false, conflicts: [] };
    };

    expect(await syncVault('v', 'msg', undefined, o)).toBe('synced');
    expect(calls.push).toBe(1);
  });

  // Offline: both directions fail. The user asked to push, so that is the error to show.
  it('reports the push error, not the pull error, when both fail', async () => {
    clearSync('v');
    const { calls, ops: o } = ops();
    o.push = async () => {
      calls.push++;
      throw new Error('PUSH FAILED');
    };
    o.pull = async () => {
      calls.pull++;
      throw new Error('PULL FAILED');
    };

    expect(await syncVault('v', 'msg', undefined, o)).toBe('failed');

    expect(syncFor('v').error).toContain('PUSH FAILED');
    expect(syncFor('v').error).not.toContain('PULL FAILED');
  });
});

describe('pullVault', () => {
  it('never pushes, whatever happens', async () => {
    clearSync('v');
    const { calls, ops: o } = ops();
    o.pull = async () => {
      calls.pull++;
      return { merged: 2, conflicts: [], kept: [] };
    };

    expect(await pullVault('v', undefined, o)).toBe('synced');

    expect(calls.push).toBe(0);
    expect(syncFor('v').merged).toBe(2);
  });

  // git refuses to merge over uncommitted edits, and the tree is dirty precisely when the
  // user has just been typing — which is when they reach for "get changes".
  it('commits before pulling', async () => {
    clearSync('v');
    const order: string[] = [];
    const { ops: o } = ops();
    o.commit = async () => {
      order.push('commit');
      return { committed: true, conflicts: [] };
    };
    o.pull = async () => {
      order.push('pull');
      return { merged: 1, conflicts: [], kept: [] };
    };

    await pullVault('v', undefined, o);

    expect(order).toEqual(['commit', 'pull']);
  });

  it('does not pull when the commit refused', async () => {
    clearSync('v');
    const { calls, ops: o } = ops();
    o.commit = async () => {
      calls.commit++;
      throw new Error('there is already a merge to finish here');
    };

    expect(await pullVault('v', undefined, o)).toBe('failed');

    expect(calls.pull).toBe(0);
  });

  it('surfaces conflicts by name', async () => {
    clearSync('v');
    const { ops: o } = ops();
    o.pull = async () => ({ merged: 0, conflicts: ['01CCC.md'], kept: [] });

    expect(await pullVault('v', undefined, o)).toBe('conflicts');

    expect(syncFor('v').conflicts).toEqual(['01CCC.md']);
  });

  // Two vaults sync independently; one failing must not colour the other.
  it('keeps vault states separate', async () => {
    clearSync('a');
    clearSync('b');
    const { ops: good } = ops();
    const { ops: bad } = ops();
    bad.pull = async () => {
      throw new Error('offline');
    };

    await pullVault('a', undefined, good);
    await pullVault('b', undefined, bad);

    expect(syncFor('a').phase).toBe('synced');
    expect(syncFor('b').phase).toBe('failed');
  });
});

// ── A vault with no remote is not a failure ──
//
// Reported 2026-07-31: "Backup needs you: vault, notes" on a device whose vaults had no remote, and
// the reasonable reading — "so nothing backed up" — is what the message invited. Each vault is
// pushed independently, so the ones with remotes always did go; the summary just could not say so,
// because a remoteless vault came back `failed` and shared a sentence with merge conflicts.
describe('a vault with no remote', () => {
  it('reports `local`, not `failed`, when there is nowhere to push', async () => {
    const { ops: o } = ops({
      // No remote means both directions fail: nothing to push to, nothing to pull from.
      push: () => Promise.reject(new Error('no remote configured')),
      pull: () => Promise.reject(new Error('no remote configured')),
      hasRemote: async () => false,
    });
    const phase = await syncVault('personal', 'backup', undefined, o);
    expect(phase).toBe('local');
    expect(syncFor('personal').phase).toBe('local');
    // No error text: nothing is wrong, so nothing to apologise for.
    expect(syncFor('personal').error).toBeUndefined();
  });

  // **The summary says "saved here, but nowhere to send" — so somebody has to have checked.**
  // `commitStep` had `CommitResult.committed` in its hand and dropped it, and the phase's own doc
  // asserted the commit too. Observed on the phone, 2026-09-08: a vault with no remote and nothing
  // new in it reported "saved here" in the same breath as the quiet-vault chip said "38 days
  // since a save". Both cannot be true, and it was the chip that had read git.
  it('does not claim to have committed when there was nothing to commit', async () => {
    const { ops: o } = ops({
      commit: async () => ({ committed: false, conflicts: [] }),
      push: () => Promise.reject(new Error('no remote configured')),
      pull: () => Promise.reject(new Error('no remote configured')),
      hasRemote: async () => false,
    });
    expect(await syncVault('personal', 'backup', undefined, o)).toBe('local');
    expect(syncFor('personal').committed).toBe(false);
  });

  it('says it did commit when it did', async () => {
    const { ops: o } = ops({
      commit: async () => ({ committed: true, conflicts: [] }),
      push: () => Promise.reject(new Error('no remote configured')),
      pull: () => Promise.reject(new Error('no remote configured')),
      hasRemote: async () => false,
    });
    expect(await syncVault('personal', 'backup', undefined, o)).toBe('local');
    expect(syncFor('personal').committed).toBe(true);
  });

  it('still reports `failed` when the vault does have a remote', async () => {
    const { ops: o } = ops({
      push: () => Promise.reject(new Error('403 forbidden')),
      pull: () => Promise.reject(new Error('403 forbidden')),
      hasRemote: async () => true,
    });
    expect(await syncVault('personal', 'backup', undefined, o)).toBe('failed');
    // And it keeps the push error, which is the thing the user asked for.
    expect(syncFor('personal').error).toContain('403');
  });

  it('treats an unanswerable status as "has a remote", so a real failure is never softened', async () => {
    const { ops: o } = ops({
      push: () => Promise.reject(new Error('boom')),
      pull: () => Promise.reject(new Error('boom')),
      hasRemote: () => Promise.reject(new Error('status unavailable')),
    });
    // Failing open in the *reassuring* direction would hide a broken backup behind a calm sentence.
    expect(await syncVault('personal', 'backup', undefined, o)).toBe('failed');
  });

  it('does not call the slow status check on the happy path', async () => {
    let asked = 0;
    const { ops: o } = ops({
      hasRemote: async () => {
        asked += 1;
        return true;
      },
    });
    expect(await syncVault('personal', 'backup', undefined, o)).toBe('synced');
    // `backup_status` shells out `git ls-remote` per vault; paying that on every successful backup
    // would make the common case the slow one.
    expect(asked).toBe(0);
  });
});

// **A note the app brought back must reach the surface, whichever door the pull came through.**
//
// Auto-settling a delete/modify keeps the vault committing (`decisions.md`, 2026-09-07) — but it
// discards a deletion to do it, so the user has to be told or the app has quietly overruled them.
// The keep is recorded independently of the phase because one merge can do both: keep one note and
// still leave a marker conflict on another. Asserting only the happy path would pass on an
// implementation that dropped the keep whenever anything else went wrong.
//
// Proven red by moving the `kept` record inside the `merged` branch: the second case then reports
// nothing, because the pull it describes ended in `conflicts`.
describe('a note kept because the other device deleted it', () => {
  it('is recorded when the pull otherwise merges cleanly', async () => {
    clearSync('v');
    const { ops: o } = ops();
    o.pull = async () => ({ merged: 1, conflicts: [], kept: ['01KEPT.md'] });

    expect(await pullVault('v', undefined, o)).toBe('synced');
    expect(syncFor('v').kept).toEqual(['01KEPT.md']);
  });

  it('is still recorded when another note in the same merge conflicts', async () => {
    clearSync('v');
    const { ops: o } = ops();
    o.pull = async () => ({ merged: 0, conflicts: ['01CONFLICT.md'], kept: ['01KEPT.md'] });

    expect(await pullVault('v', undefined, o)).toBe('conflicts');
    expect(syncFor('v').conflicts).toEqual(['01CONFLICT.md']);
    expect(syncFor('v').kept).toEqual(['01KEPT.md']);
  });
});

// ── The indicator that says "wait" ──
//
// **`syncing()` was written for this and never called.** Its own docstring says it is "what a
// global 'syncing…' indicator reads"; a grep for consumers returned only the definition. So the
// app did every slow thing — commit, pull over the network, push — with no sign on screen that
// anything was happening.
//
// Reported 2026-09-08, from a phone: *"I pressed get their changes. But on timeline and agenda I
// do not see what I see on my laptop… Ok, I see them only now (time issue with pull I guess). Some
// icon rotating like a wheel should be visually present."* The data was never wrong — the pull was
// simply still running, and the app's one clue that it had started (the "get changes" chip) is
// **cleared at the start of the handler**, so pressing it made the only evidence disappear.
const tick = () => new Promise((r) => setTimeout(r, 0));

describe('the working indicator', () => {
  it('names nothing when nothing is in flight', () => {
    expect(syncingPhase()).toBeNull();
    expect(busyLabel(null)).toBe('');
  });

  it('says what is happening while a pull runs, and stops when it ends', async () => {
    let release!: () => void;
    const gate = new Promise<void>((r) => (release = r));
    const { ops: o } = ops({
      pull: async () => {
        await gate;
        return { merged: 0, conflicts: [], kept: [] };
      },
    });

    const run = pullVault('slow-pull', undefined, o);
    await tick();
    expect(syncingPhase()).toBe('pulling');
    expect(busyLabel(syncingPhase())).toBe('Getting changes…');

    release();
    await run;
    expect(syncingPhase()).toBeNull();
    clearSync('slow-pull');
  });

  it('says so while the slow half of a backup is running', async () => {
    let release!: () => void;
    const gate = new Promise<void>((r) => (release = r));
    const { ops: o } = ops({ push: async () => void (await gate) });

    const run = syncVault('slow-push', 'm', undefined, o);
    await tick();
    expect(busyLabel(syncingPhase())).toBe('Sending…');

    release();
    await run;
    expect(syncingPhase()).toBeNull();
    clearSync('slow-push');
  });

  it('names the step the user is actually waiting on when vaults are at different ones', async () => {
    // Committing is local and quick; the network steps are the wait. With two vaults in flight the
    // indicator has one line to spend, so it spends it on the slow one.
    let release!: () => void;
    const gate = new Promise<void>((r) => (release = r));
    const slow = ops({
      pull: async () => {
        await gate;
        return { merged: 0, conflicts: [], kept: [] };
      },
    });
    const stuck = ops({
      commit: async () => {
        await gate;
        return { committed: true, conflicts: [] };
      },
    });

    const a = pullVault('net', undefined, slow.ops);
    const b = pullVault('local', undefined, stuck.ops);
    await tick();
    expect(syncingPhase()).toBe('pulling');

    release();
    await Promise.all([a, b]);
    expect(syncingPhase()).toBeNull();
    clearSync('net');
    clearSync('local');
  });

  it('goes quiet when a sync fails, so the spinner never outlives the work', async () => {
    const { ops: o } = ops({
      pull: async () => {
        throw new Error('no');
      },
    });
    await pullVault('broken', undefined, o);
    expect(syncingPhase()).toBeNull();
    expect(syncFor('broken').phase).toBe('failed');
    clearSync('broken');
  });
});

// ── Saying why a send failed, in words the reader has ──
//
// What the owner saw on the phone, 2026-09-08, as the whole message:
//
//   io error cannot push because a reference that you are trying to update on the remote
//   contains commits that are not present locally
//
// They worked out what it meant — *"(which makes sense)"* — and then did the right thing by hand.
// That is the app handing its reader a puzzle it had already solved: this state has exactly one
// remedy, the app knows it, and there is a button for it. `decisions.md` (*the app speaks the
// user's words, not git's*) allows the git wording in a *diagnostic*; it was the headline.
describe('why a send failed', () => {
  it('turns the one error with an obvious remedy into that remedy', () => {
    const said = plainError(
      'io error cannot push because a reference that you are trying to update on the remote ' +
        'contains commits that are not present locally',
    );
    expect(said).toBe(
      "The other device has changes you don't have yet — get their changes first, then send.",
    );
  });

  it("recognises the same refusal in git's other spellings of it", () => {
    // Two backends and several git versions word this differently; the state is identical.
    for (const raw of [
      'failed to push some refs: non-fast-forward',
      'Updates were rejected because the remote contains work that you do not have locally. ' +
        'Integrate the remote changes (e.g. hint: git pull) before pushing again.',
      'cannot push because a reference that you are trying to update on the remote contains ' +
        'commits that are not present locally',
    ]) {
      expect(plainError(raw)).toMatch(/get their changes first/i);
    }
  });

  it('says a dropped connection is safe to retry, and keeps the diagnosis', () => {
    // Reported from the phone, 2026-09-09, on the first backup carrying a photo. OpenSSL's
    // `8` is ERR_LIB_SYS and `0x20` is EPIPE — true, and useless to someone who wanted a backup.
    const said = plainError('SSL error: error:80000020: system library::Broken pipe');
    expect(said).toMatch(/connection dropped part-way through sending/i);
    expect(said).toMatch(/nothing was lost/i);
    expect(said).toMatch(/safe to try again/i);
    // The detail survives, because a dropped connection has several causes and the wording is
    // what separates them — unlike "the other device is ahead", which has exactly one remedy.
    expect(said).toContain('error:80000020');
  });

  it('recognises the other spellings of a connection that went away', () => {
    for (const raw of [
      'failed to send request: Connection reset by peer',
      'early EOF',
      'operation timed out after 30000 milliseconds',
      'could not resolve host: github.com',
    ]) {
      expect(plainError(raw)).toMatch(/connection dropped part-way|could not resolve/i);
    }
  });

  it('does not mistake a refusal or a conflict for a dead wire', () => {
    // The two must not collapse: one says "try again", the other says "do something first".
    expect(plainError('failed to push some refs: non-fast-forward')).toMatch(
      /get their changes first/i,
    );
    expect(plainError('authentication failed — check the token')).toBe(
      'authentication failed — check the token',
    );
  });

  it('leaves anything it does not recognise exactly as it was', () => {
    // The deliberate escape hatch: an error nobody has written a sentence for must reach the user
    // verbatim rather than be flattened into a reassuring shrug. Losing the only diagnosis of an
    // unknown fault is a worse failure than showing an ugly one.
    expect(plainError('io error: disk quota exceeded')).toBe('io error: disk quota exceeded');
    expect(plainError('')).toBe('');
  });

  it('does not mistake an unrelated mention of the remote for the ahead-of-us case', () => {
    // It is now recognised as a dead wire instead, which is what it is — but it must never be
    // reported as "the other device has changes you don't have".
    expect(plainError('could not resolve host: github.com')).not.toMatch(/get their changes/i);
  });
});

// ── Staging a backlog, from whichever button ──
//
// The first attempt put this loop in the Backup panel's own button. The button most people press
// is the one in the toolbar, which calls `syncVault` directly — so the owner ticked the box,
// pressed Back up, and got the same broken pipe with no steps at all. The option was in one place
// and the action in another.
//
// It lives in `syncVault` now, which every backup path goes through. These tests are about that
// property: the caller does not opt in, does not pass anything, and still gets steps.
describe('a backlog too big for one push', () => {
  const plan = [
    { cap: 100, count: 2, bytes: 150 },
    { cap: 900, count: 1, bytes: 900 },
  ];

  it('is sent in steps without the caller asking for it', async () => {
    const caps: (number | undefined)[] = [];
    const { ops: o } = ops({
      batches: async () => plan,
      commit: async (_m: string, _v: string, cap?: number) => {
        caps.push(cap);
        return { committed: true, conflicts: [] };
      },
    });

    expect(await syncVault('big', 'backup', undefined, o)).toBe('synced');
    // One pass per step, each with its own rising cap, then a final uncapped pass so anything
    // written while it ran still goes.
    expect(caps).toEqual([100, 900, undefined]);
    clearSync('big');
  });

  it('stops where it failed, and says how far it got', async () => {
    // Each step is a completed push, so the ones before the failure are already sent. What must
    // not happen is the loop carrying on into steps that cannot land.
    let n = 0;
    const { ops: o } = ops({
      batches: async () => plan,
      push: async () => {
        n += 1;
        if (n > 1) throw new Error('SSL error: error:80000020: system library::Broken pipe');
      },
    });

    expect(await syncVault('halted', 'backup', undefined, o)).toBe('failed');
    expect(n).toBe(2);
    expect(syncFor('halted').step).toBe(2);
    expect(syncFor('halted').steps).toBe(3);
    clearSync('halted');
  });

  it('a vault with nothing waiting is one push, exactly as before', async () => {
    const caps: (number | undefined)[] = [];
    const { ops: o } = ops({
      batches: async () => [],
      commit: async (_m: string, _v: string, cap?: number) => {
        caps.push(cap);
        return { committed: true, conflicts: [] };
      },
    });
    await syncVault('small', 'backup', undefined, o);
    expect(caps).toEqual([undefined]);
    clearSync('small');
  });

  it('a single step is one push too — a loop there would be ceremony', async () => {
    const caps: (number | undefined)[] = [];
    const { ops: o } = ops({
      batches: async () => [{ cap: 500, count: 1, bytes: 500 }],
      commit: async (_m: string, _v: string, cap?: number) => {
        caps.push(cap);
        return { committed: true, conflicts: [] };
      },
    });
    await syncVault('one', 'backup', undefined, o);
    expect(caps).toEqual([undefined]);
    clearSync('one');
  });
});
