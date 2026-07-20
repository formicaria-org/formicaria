// The sync sequence, driven through every branch with fake git operations.
//
// These are the states that matter and that a real remote will not hold still for: a push
// rejected because someone else pushed, a merge that genuinely disagrees, a push that fails
// for a reason pulling cannot fix. Each has a rule attached, and the rules are the reason
// this module exists rather than a `commit().then(push)`.

import { describe, expect, it, vi } from 'vitest';
import { clearSync, pullVault, syncFor, syncVault, type SyncOps } from './sync.svelte';

/** A fake git. `pushes` counts attempts, which is how "exactly one retry" is asserted. */
function ops(over: Partial<SyncOps> = {}) {
  const calls = { commit: 0, push: 0, pull: 0 };
  const base: SyncOps = {
    commit: async () => {
      calls.commit++;
      return { committed: true, conflicts: [] };
    },
    push: async () => void calls.push++,
    pull: async () => {
      calls.pull++;
      return { merged: 0, conflicts: [] };
    },
  };
  return { calls, ops: { ...base, ...over } as SyncOps };
}

/** A push that is rejected the first N times, then succeeds. */
function rejectingPush(times: number, calls: { push: number }) {
  return async () => {
    calls.push++;
    if (calls.push <= times) throw new Error('the remote has changes you don\'t have');
    };
}

describe('syncVault', () => {
  it('commits and pushes when the remote has not moved', async () => {
    clearSync('v');
    const { calls, ops: o } = ops();

    expect(await syncVault('v', 'msg', undefined, o)).toBe('synced');

    expect(calls).toEqual({ commit: 1, push: 1, pull: 0 });
    expect(syncFor('v').phase).toBe('synced');
  });

  // The sequence this module exists for.
  it('heals a moved remote: rejected push -> pull -> push once more', async () => {
    clearSync('v');
    const { calls, ops: o } = ops();
    o.push = rejectingPush(1, calls);
    o.pull = async () => {
      calls.pull++;
      return { merged: 3, conflicts: [] };
    };
    const onChanged = vi.fn();

    expect(await syncVault('v', 'msg', onChanged, o)).toBe('synced');

    expect(calls.push).toBe(2);
    expect(calls.pull).toBe(1);
    expect(onChanged).toHaveBeenCalledOnce();
    expect(syncFor('v').merged).toBe(3);
  });

  // The rule that makes this safe. Conflict markers are not content.
  it('never pushes after a pull that conflicted', async () => {
    clearSync('v');
    const { calls, ops: o } = ops();
    o.push = rejectingPush(1, calls);
    o.pull = async () => {
      calls.pull++;
      return { merged: 0, conflicts: ['01AAA.md', '01BBB.md'] };
    };

    expect(await syncVault('v', 'msg', undefined, o)).toBe('conflicts');

    // The second push must never happen.
    expect(calls.push).toBe(1);
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
      return { merged: 1, conflicts: [] };
    };

    expect(await syncVault('v', 'msg', undefined, o)).toBe('failed');

    // Tried twice, then stopped.
    expect(calls.push).toBe(2);
    expect(calls.pull).toBe(1);
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
      return { merged: 2, conflicts: [] };
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
      return { merged: 1, conflicts: [] };
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
    o.pull = async () => ({ merged: 0, conflicts: ['01CCC.md'] });

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
