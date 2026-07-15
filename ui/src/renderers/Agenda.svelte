<script lang="ts">
  import { urgency, relativeDue, urgencyLabel, URGENCY_ORDER } from '../lib/urgency';
  import { formatStamp } from '../lib/stamp';
  import type { ObjectMeta } from '../lib/types';

  // The closest-deadline view. Cards arrive already filtered (dated, open) and
  // sorted (soonest first) by the query layer; this renderer only paints the
  // derived urgency and flags hard deadlines. No status literal appears here.
  let { cards, onopen }: { cards: ObjectMeta[]; onopen: (id: string) => void } = $props();

  // Group into urgency bands (Things-style "Overdue / This week / Later"). The
  // incoming sort is preserved within each band; empty bands are dropped. Labels
  // come from urgency.ts so no scheduling literal lives in this renderer.
  let groups = $derived(
    URGENCY_ORDER.map((u) => ({
      u,
      label: urgencyLabel(u),
      items: cards.filter((c) => urgency(c.due) === u),
    })).filter((g) => g.items.length > 0),
  );
</script>

<div class="agenda">
  {#if cards.length === 0}
    <p class="empty">Nothing on the horizon — no dated, open items.</p>
  {:else}
    <div class="list">
      {#each groups as group (group.u)}
        <section class="band" data-urgency={group.u}>
          <h3 class="section">
            <span class="section-dot" aria-hidden="true"></span>
            {group.label}
            <span class="section-count">{group.items.length}</span>
          </h3>
          {#each group.items as card (card.id)}
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
                <span class="date">{formatStamp(card.due)}</span>
                <span class="rel">{relativeDue(card.due)}</span>
              </span>
            </button>
          {/each}
        </section>
      {/each}
    </div>
  {/if}
</div>

<style>
  .agenda {
    height: 100%;
    overflow-y: auto;
    padding: var(--space-4);
    box-sizing: border-box;
  }
  .list {
    margin: 0 auto;
    max-width: 46rem;
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }
  .band {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  /* Sticky urgency header; its dot/label color inherits [data-urgency]. */
  .section {
    position: sticky;
    top: calc(var(--space-4) * -1);
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0 0 var(--space-1);
    padding: var(--space-1) var(--space-1);
    font-size: var(--text-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--urgency, var(--text-muted));
    background: var(--bg);
  }
  .section-dot {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 50%;
    background: var(--urgency, var(--text-muted));
  }
  .section-count {
    color: var(--text-subtle);
    font-weight: 500;
  }
  .row {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    background: var(--surface-elevated);
    border: 1px solid var(--border);
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
  .row + .row {
    margin-top: var(--space-1);
  }
  .row:hover {
    border-color: var(--accent);
    box-shadow: var(--shadow-sm);
  }
  /* The urgency dot is colored by [data-urgency] in the theme (app.css). */
  .marker {
    width: 0.6rem;
    height: 0.6rem;
    border-radius: 50%;
    background: var(--urgency, var(--text-muted));
  }
  .what {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .title {
    color: var(--text);
    font-size: var(--text-sm);
    overflow-wrap: anywhere;
  }
  .tags {
    display: flex;
    gap: var(--space-1);
    flex-wrap: wrap;
  }
  .tag {
    font-size: var(--text-xs);
    padding: 0.02rem 0.35rem;
    border-radius: var(--radius-pill);
    background: var(--tag-bg);
    color: var(--tag-fg);
  }
  .when {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    white-space: nowrap;
  }
  .hard {
    color: var(--due-hard-fg);
    font-size: 0.7rem;
  }
  .date {
    font-size: var(--text-sm);
    color: var(--text);
  }
  .rel {
    font-size: var(--text-xs);
    color: var(--text-muted);
    min-width: 5.5rem;
    text-align: right;
  }
  .empty {
    padding: var(--space-6);
    color: var(--text-muted);
    text-align: center;
  }
</style>
