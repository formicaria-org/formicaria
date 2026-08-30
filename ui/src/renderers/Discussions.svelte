<script lang="ts">
  // The ongoing discussions, at a glance — title + vault + who has posted, most-recently-active
  // first (the server sorts). A discussion is a note that is the root of its own thread; clicking
  // one opens it, and the pane renders it as a conversation rather than a document. The vault +
  // contributor filters apply through the same `shown` predicate every other view uses. No status
  // literal → renderer-safe.
  import VaultBadge from '../lib/VaultBadge.svelte';
  import { dayHeading } from '../lib/calendar';
  import type { DiscussionSummary } from '../lib/types';
  import EmptyState from '../lib/EmptyState.svelte';

  let {
    discussions,
    shown,
    onopen,
  }: {
    discussions: DiscussionSummary[];
    shown: (n: { id: string; vault: string }) => boolean;
    onopen: (id: string) => void;
  } = $props();

  const rows = $derived(discussions.filter((d) => shown({ id: d.id, vault: d.vault })));

  // "Who left a message" — the whole point of the at-a-glance row. Empty until someone posts.
  function who(d: DiscussionSummary): string {
    if (d.participants.length === 0) return 'no messages yet';
    return d.participants.map((p) => p.name).join(', ');
  }
</script>

<div class="discussions">
  {#if rows.length === 0}
    <EmptyState
      icon="chat"
      title="No discussions yet"
      hint="Start one from + → New discussion, or leave a comment on any note." />
  {:else}
    <div class="feed">
      {#each rows as d (d.id)}
        <button class="row" onclick={() => onopen(d.id)} title={d.title ?? 'Untitled discussion'}>
          <span class="head">
            <span class="title">{d.title || 'Untitled discussion'}</span>
            <VaultBadge vault={d.vault} />
          </span>
          <span class="meta">
            <span class="count">{d.count} {d.count === 1 ? 'message' : 'messages'}</span>
            <span class="who">{who(d)}</span>
            <span class="when">{dayHeading(d.last_activity.slice(0, 10))}</span>
          </span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .discussions {
    height: 100%;
    overflow-y: auto;
    padding: 1rem;
    box-sizing: border-box;
  }
  .feed {
    margin: 0 auto;
    max-width: 46rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .row {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    padding: 0.7rem 0.9rem;
    background: var(--card-bg);
    border: 1px solid var(--card-border);
    border-radius: var(--radius-md);
    cursor: pointer;
    text-align: left;
    font: inherit;
    color: inherit;
    width: 100%;
  }
  .row:hover {
    border-color: var(--accent);
    box-shadow: var(--shadow-sm);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }
  .title {
    flex: 1;
    min-width: 0;
    color: var(--text);
    font-size: 0.95rem;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.5rem;
    font-size: 0.78rem;
    color: var(--muted);
  }
  .count {
    color: var(--accent);
  }
  .who {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
