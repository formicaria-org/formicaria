<script lang="ts">
  import { urgency, relativeDue } from '../lib/urgency';
  import type { ObjectMeta } from '../lib/types';

  // The closest-deadline view. Cards arrive already filtered (dated, open) and
  // sorted (soonest first) by the query layer; this renderer only paints the
  // derived urgency and flags hard deadlines. No status literal appears here.
  let { cards, onopen }: { cards: ObjectMeta[]; onopen: (id: string) => void } = $props();
</script>

<div class="agenda">
  {#if cards.length === 0}
    <p class="empty">Nothing on the horizon — no dated, open items.</p>
  {:else}
    <div class="list">
      {#each cards as card (card.id)}
        <button class="row" data-urgency={urgency(card.due)} onclick={() => onopen(card.id)}>
          <span class="marker" aria-hidden="true"></span>
          <span class="what">
            <span class="title">{card.title ?? card.preview ?? card.id}</span>
            {#if card.tags.length}
              <span class="tags">
                {#each card.tags as tag (tag)}<span class="tag">{tag}</span>{/each}
              </span>
            {/if}
          </span>
          <span class="when">
            {#if card.hard}<span class="hard" title="hard deadline">◆</span>{/if}
            <span class="date">{card.due}</span>
            <span class="rel">{relativeDue(card.due)}</span>
          </span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .agenda {
    height: 100%;
    overflow-y: auto;
    padding: 1rem;
    box-sizing: border-box;
  }
  .list {
    margin: 0 auto;
    max-width: 46rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .row {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 0.7rem;
    padding: 0.55rem 0.8rem;
    background: var(--card-bg);
    border: 1px solid var(--card-border);
    border-radius: 9px;
    cursor: pointer;
    text-align: left;
    font: inherit;
    color: inherit;
    width: 100%;
  }
  .row:hover {
    border-color: var(--accent);
  }
  /* The urgency dot is colored by [data-urgency] in the theme (app.css). */
  .marker {
    width: 0.6rem;
    height: 0.6rem;
    border-radius: 50%;
    background: var(--urgency, var(--muted));
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
  .tags {
    display: flex;
    gap: 0.3rem;
    flex-wrap: wrap;
  }
  .tag {
    font-size: 0.68rem;
    padding: 0.02rem 0.35rem;
    border-radius: 999px;
    background: var(--tag-bg);
    color: var(--tag-fg);
  }
  .when {
    display: flex;
    align-items: baseline;
    gap: 0.45rem;
    white-space: nowrap;
  }
  .hard {
    color: var(--due-hard-fg);
    font-size: 0.7rem;
  }
  .date {
    font-size: 0.82rem;
    color: var(--text);
  }
  .rel {
    font-size: 0.72rem;
    color: var(--muted);
    min-width: 5.5rem;
    text-align: right;
  }
  .empty {
    padding: 2rem;
    color: var(--muted);
    text-align: center;
  }
</style>
