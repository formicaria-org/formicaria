<script lang="ts">
  import type { ObjectMeta } from '../lib/types';

  // Results renderer for full-text search. The same ObjectMeta every other view
  // receives; it paints a type chip, the title/preview, and tags, and opens the
  // note on click. No status literal appears here (the CI grep forbids them in
  // renderers) — it only reads generic card fields.
  let {
    cards,
    query,
    onopen,
  }: { cards: ObjectMeta[]; query: string; onopen: (id: string) => void } = $props();
</script>

<div class="results">
  {#if !query.trim()}
    <p class="empty">Search your notes, tasks, meetings, and ingested documents.</p>
  {:else if cards.length === 0}
    <p class="empty">No matches for “{query}”.</p>
  {:else}
    <div class="list">
      {#each cards as card (card.id)}
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
  {/if}
</div>

<style>
  .results {
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
    grid-template-columns: auto 1fr;
    align-items: start;
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
