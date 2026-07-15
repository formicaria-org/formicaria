<script lang="ts">
  import { dayHeading } from '../lib/calendar';
  import type { ObjectMeta } from '../lib/types';

  // A Logseq-style journal: every note under the day it was created, newest day
  // first. Cards arrive already sorted newest-created-first (the `recent` query),
  // so same-day notes are contiguous and we just partition them into day buckets
  // in order. No status literal here — renderer-safe.
  let { cards, onopen }: { cards: ObjectMeta[]; onopen: (id: string) => void } = $props();

  let days = $derived.by(() => {
    const out: { key: string; heading: string; items: ObjectMeta[] }[] = [];
    for (const c of cards) {
      const key = c.created.slice(0, 10);
      let bucket = out[out.length - 1];
      if (!bucket || bucket.key !== key) {
        bucket = { key, heading: dayHeading(key), items: [] };
        out.push(bucket);
      }
      bucket.items.push(c);
    }
    return out;
  });
</script>

<div class="timeline">
  {#if cards.length === 0}
    <p class="empty">No notes yet — capture one above.</p>
  {:else}
    <div class="feed">
      {#each days as day (day.key)}
        <section class="day">
          <h2 class="heading">
            <span>{day.heading}</span>
            <span class="count">{day.items.length}</span>
          </h2>
          <div class="items">
            {#each day.items as card (card.id)}
              <button class="row" data-type={card.type} onclick={() => onopen(card.id)}>
                <span class="type" data-type={card.type}>{card.type}</span>
                <span class="what">
                  <span class="title">{card.title ?? card.preview ?? card.id}</span>
                  {#if card.title && card.preview}<span class="preview">{card.preview}</span>{/if}
                  {#if card.tags.length}
                    <span class="tags">
                      {#each card.tags as tag (tag)}<span class="tag">{tag}</span>{/each}
                    </span>
                  {/if}
                </span>
              </button>
            {/each}
          </div>
        </section>
      {/each}
    </div>
  {/if}
</div>

<style>
  .timeline {
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
    display: grid;
    grid-template-columns: auto 1fr;
    align-items: start;
    gap: 0.7rem;
    padding: 0.55rem 0.8rem;
    background: var(--card-bg);
    border: 1px solid var(--card-border);
    border-radius: var(--radius-md);
    cursor: pointer;
    text-align: left;
    font: inherit;
    color: inherit;
    width: 100%;
    transition:
      border-color var(--dur-fast) var(--ease),
      box-shadow var(--dur-fast) var(--ease);
  }
  .row:hover {
    border-color: var(--accent);
    box-shadow: var(--shadow-sm);
  }
  .type {
    font-size: 0.66rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--muted);
    border: 1px solid var(--card-border);
    border-radius: 999px;
    padding: 0.05rem 0.45rem;
    margin-top: 0.1rem;
  }
  .what {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .title {
    color: var(--text);
    font-size: 0.9rem;
    overflow-wrap: anywhere;
  }
  .preview {
    font-size: 0.76rem;
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .tags {
    display: flex;
    gap: 0.3rem;
    flex-wrap: wrap;
    margin-top: 0.1rem;
  }
  .tag {
    font-size: 0.68rem;
    padding: 0.02rem 0.35rem;
    border-radius: 999px;
    background: var(--tag-bg);
    color: var(--tag-fg);
  }
  .empty {
    padding: 2rem;
    color: var(--muted);
    text-align: center;
  }
</style>
