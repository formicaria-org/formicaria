<script lang="ts">
  import VaultBadge from '../lib/VaultBadge.svelte';
  import EditedBy from '../lib/EditedBy.svelte';
  import { lastEditFor } from '../lib/activity.svelte';
  import type { ObjectMeta } from '../lib/types';
  import EmptyState from '../lib/EmptyState.svelte';

  // Results renderer for full-text search. The same ObjectMeta every other view
  // receives; it paints the title/preview and tags, an "asset" chip only when the
  // hit is an ingested file (plain notes carry no redundant type label), and opens
  // the note on click. No status literal appears here (the CI grep forbids them in
  // renderers) — it only reads generic card fields.
  // **`query` is optional, and its absence is a different thing from an empty one.** A search pane
  // always has a query box, so `''` means "you have not typed yet" and the right answer is a
  // prompt. A saved `.view` drawn through this renderer has no box at all — it is a filter someone
  // wrote, already applied — so a prompt to type would be an instruction the user cannot follow.
  // Undefined means "this is a list, not a search": no prompt, and an empty result says the filter
  // matched nothing rather than blaming the reader for typing badly.
  let {
    cards,
    query,
    onopen,
  }: { cards: ObjectMeta[]; query?: string; onopen: (id: string) => void } = $props();
  const searching = $derived(query !== undefined);
</script>

<div class="results">
  {#if searching && !query!.trim()}
    <EmptyState
      icon="search"
      title="Search your notes"
      hint="Every word of every note, and the names of the files you have added."
    />
  {:else if cards.length === 0}
    {#if searching}
      <EmptyState
        icon="search"
        title="No matches for “{query}”"
        hint="Try fewer words, or a different spelling."
      />
    {:else}
      <EmptyState icon="search" title="Nothing here yet" hint="This view has no notes in it." />
    {/if}
  {:else}
    <div class="list">
      {#each cards as card (card.id)}
        <button class="row" data-type={card.type} onclick={() => onopen(card.id)}>
          <span class="what">
            <span class="title">
              {#if card.type === 'asset'}<span class="type">asset</span>{/if}
              {card.title ?? card.preview ?? card.id}
            </span>
            {#if card.title && card.preview}<span class="preview">{card.preview}</span>{/if}
            {#if card.vault || card.tags.length}
              <span class="tags">
                <VaultBadge vault={card.vault} />
                <EditedBy edit={lastEditFor(card.id)} />
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
    display: block;
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
    display: inline-block;
    font-size: 0.66rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--muted);
    border: 1px solid var(--card-border);
    border-radius: 999px;
    padding: 0.05rem 0.45rem;
    margin-right: 0.4rem;
    vertical-align: middle;
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
</style>
