<script lang="ts">
  // Show a proposal's change for review: the files it touches and its unified diff against `main`,
  // fetched via `proposalDiff`. A proposal whose branch is gone (merged or deleted) is reported
  // plainly — "nothing to show" — because the proposal note outlives its branch.
  import { proposalDiff, acceptProposal } from './ipc';
  import type { ProposalDiff } from './types';

  // `onaccepted` lets the parent (the Collaboration list) refresh once a proposal is merged.
  let { id, onaccepted }: { id: string; onaccepted?: () => void } = $props();

  let diff = $state<ProposalDiff | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(true);

  // The accept (merge) round-trip — the GUI's stand-in for `git merge`, since a UI-only user has none.
  let accepting = $state(false);
  let acceptMsg = $state<string | null>(null);

  async function accept() {
    accepting = true;
    acceptMsg = null;
    try {
      const { outcome } = await acceptProposal(id);
      if (outcome === 'conflicted') {
        acceptMsg = "This proposal doesn't merge cleanly onto the current note — main was left unchanged. Ask the author to redo it against the latest, or resolve it after a sync.";
      } else {
        // 'merged' or 'already_gone' → it is on main now. Re-fetch: the branch is gone, so the diff
        // flips to the "merged" message, and let the parent refresh its list.
        acceptMsg = outcome === 'already_gone' ? 'Already merged.' : 'Merged into main.';
        await refetch();
        onaccepted?.();
      }
    } catch (e) {
      acceptMsg = `Couldn't accept: ${e instanceof Error ? e.message : String(e)}`;
    } finally {
      accepting = false;
    }
  }

  async function refetch() {
    try {
      diff = await proposalDiff(id);
    } catch (e) {
      error = String(e);
    }
  }

  // Re-fetch whenever the id changes; ignore a stale response if it swaps mid-flight.
  $effect(() => {
    const which = id;
    let cancelled = false;
    loading = true;
    error = null;
    diff = null;
    proposalDiff(which)
      .then((d) => {
        if (!cancelled) {
          diff = d;
          loading = false;
        }
      })
      .catch((e) => {
        if (!cancelled) {
          error = String(e);
          loading = false;
        }
      });
    return () => {
      cancelled = true;
    };
  });

  const lines = $derived((diff?.patch ?? '').split('\n'));

  // A unified-diff line's role, for colouring. `+++`/`---` are file headers, not adds/deletes.
  function role(line: string): string {
    if (line.startsWith('@@')) return 'hunk';
    if (line.startsWith('+++') || line.startsWith('---') || line.startsWith('diff ')) return 'meta';
    if (line.startsWith('+')) return 'add';
    if (line.startsWith('-')) return 'del';
    return 'ctx';
  }
</script>

<div class="review">
  {#if loading}
    <p class="muted">Loading the proposed change…</p>
  {:else if error}
    <p class="error">Couldn't load the diff: {error}</p>
  {:else if diff && !diff.exists}
    <p class="muted">
      This proposal's branch is gone — merged or deleted. There's nothing to show; the discussion
      lives on.
    </p>
  {:else if diff}
    {#if diff.files.length}
      <p class="files">
        {diff.files.length} file{diff.files.length === 1 ? '' : 's'}: {diff.files.join(', ')}
      </p>
    {/if}
    <pre class="diff">{#each lines as line}<code class="line {role(line)}">{line}</code>{/each}</pre>
    <div class="review-actions">
      <button class="accept" onclick={accept} disabled={accepting}>
        {accepting ? 'Merging…' : 'Accept & merge'}
      </button>
    </div>
  {/if}

  {#if acceptMsg}
    <p class="accept-msg" class:muted={!acceptMsg.startsWith("Couldn't") && !acceptMsg.startsWith('This')}>
      {acceptMsg}
    </p>
  {/if}
</div>

<style>
  .review {
    padding: 0.25rem 0 0.5rem;
  }
  .review-actions {
    display: flex;
    justify-content: flex-end;
    margin-top: 0.5rem;
  }
  button.accept {
    background: var(--accent, #6639ba);
    border: 1px solid var(--accent, #6639ba);
    color: #fff;
    padding: 4px 12px;
    border-radius: var(--radius-2, 6px);
    font-size: 0.85rem;
    cursor: pointer;
  }
  button.accept:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .accept-msg {
    margin: 0.4rem 0 0;
    font-size: 0.9em;
    color: var(--danger, #c0392b);
  }
  .accept-msg.muted {
    color: var(--success, #1a7f37);
  }
  .muted {
    color: var(--muted);
    font-size: 0.9em;
  }
  .error {
    color: var(--danger, #c0392b);
    font-size: 0.9em;
  }
  .files {
    color: var(--muted);
    font-size: 0.85em;
    margin: 0 0 0.35rem;
    word-break: break-all;
  }
  .diff {
    margin: 0;
    padding: 0.5rem 0.6rem;
    overflow-x: auto;
    background: var(--surface-2, rgba(127, 127, 127, 0.08));
    border-radius: 6px;
    font-size: 0.82em;
    line-height: 1.4;
  }
  .line {
    display: block;
    white-space: pre;
  }
  .line.add {
    color: #1a7f37;
    background: rgba(46, 160, 67, 0.12);
  }
  .line.del {
    color: #cf222e;
    background: rgba(207, 34, 46, 0.12);
  }
  .line.hunk {
    color: var(--accent, #6639ba);
  }
  .line.meta {
    color: var(--muted);
  }
  @media (prefers-color-scheme: dark) {
    .line.add {
      color: #3fb950;
    }
    .line.del {
      color: #f85149;
    }
  }
</style>
