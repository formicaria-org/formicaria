<script lang="ts">
  // The git activity stream — "who changed which notes, when" — as a first-class view. Reads the
  // reactive activity module directly (App refreshes it alongside the feeds), so it needs no feed
  // fetch of its own. One row per recently-edited note, newest-first, grouped by day like the
  // timeline. The vault + contributor filters apply through the same `shown` predicate every other
  // view uses, so hiding a contributor also empties their rows here. No status literal → renderer-safe.
  import VaultBadge from '../lib/VaultBadge.svelte';
  import EditedBy from '../lib/EditedBy.svelte';
  import { activityEvents } from '../lib/activity.svelte';
  import { dayHeading } from '../lib/calendar';
  import type { EditEvent } from '../lib/types';

  let {
    shown,
    onopen,
  }: {
    shown: (n: { id: string; vault: string }) => boolean;
    onopen: (id: string) => void;
  } = $props();

  const days = $derived.by(() => {
    const out: { key: string; heading: string; items: EditEvent[] }[] = [];
    for (const e of activityEvents()) {
      if (!shown(e)) continue;
      const key = e.time.slice(0, 10);
      let bucket = out[out.length - 1];
      if (!bucket || bucket.key !== key) {
        bucket = { key, heading: dayHeading(key), items: [] };
        out.push(bucket);
      }
      bucket.items.push(e);
    }
    return out;
  });
</script>

<div class="activity">
  {#if days.length === 0}
    <p class="empty">No history yet — edits show up here once notes are committed to git.</p>
  {:else}
    <div class="feed">
      {#each days as day (day.key)}
        <section class="day">
          <h2 class="heading">
            <span>{day.heading}</span>
            <span class="count">{day.items.length}</span>
          </h2>
          <div class="items">
            {#each day.items as e (e.id)}
              <button class="row" onclick={() => onopen(e.id)} title={e.title ?? e.id}>
                <EditedBy edit={e} />
                <span class="title">{e.title ?? e.id}</span>
                <VaultBadge vault={e.vault} />
              </button>
            {/each}
          </div>
        </section>
      {/each}
    </div>
  {/if}
</div>

<style>
  .activity {
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
    gap: 1.1rem;
  }
  .heading {
    position: sticky;
    top: 0;
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
    margin: 0 0 0.4rem;
    padding: 0.2rem 0;
    font-size: 0.82rem;
    font-weight: 600;
    color: var(--accent);
    background: var(--bg);
    border-bottom: 1px solid var(--column-border);
  }
  .count {
    font-size: 0.7rem;
    font-weight: 400;
    color: var(--muted);
  }
  .items {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.5rem 0.8rem;
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
  .title {
    flex: 1;
    min-width: 0;
    color: var(--text);
    font-size: 0.9rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .empty {
    padding: 2rem;
    color: var(--muted);
    text-align: center;
  }
</style>
