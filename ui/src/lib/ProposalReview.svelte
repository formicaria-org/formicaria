<script lang="ts">
  // Show a proposal's change for review: the files it touches and its unified diff against `main`,
  // fetched via `proposalDiff`. A proposal whose branch is gone (merged or deleted) is reported
  // plainly — "nothing to show" — because the proposal note outlives its branch.
  import { proposalDiff } from './ipc';
  import type { ProposalDiff } from './types';

  let { id }: { id: string } = $props();

  let diff = $state<ProposalDiff | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(true);

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
  {/if}
</div>

<style>
  .review {
    padding: 0.25rem 0 0.5rem;
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
