<script lang="ts">
  import AssetMissing from '../lib/AssetMissing.svelte';
  import type { ObjectMeta } from '../lib/types';

  // The second renderer. It receives the same ObjectMeta the board does — the
  // only difference from the board is the query behind it (type = asset) and the
  // shape it paints (a grid, not columns). No status literal, no bespoke query.
  let { cards, onopen }: { cards: ObjectMeta[]; onopen: (id: string) => void } = $props();
</script>

<div class="gallery">
  {#if cards.length === 0}
    <p class="empty">No assets yet. Ingest media with `fm add` (hashing, thumbnails).</p>
  {:else}
    {#each cards as card (card.id)}
      <button class="tile" data-type={card.type} onclick={() => onopen(card.id)}>
        <span class="thumb">
          <!-- No blob resolver yet (asset_status / resolve_asset land later), so
               every asset shows the shared placeholder — graceful absence. -->
          <AssetMissing label={card.title ?? card.preview} />
        </span>
        <span class="caption">{card.title ?? card.preview ?? card.id}</span>
      </button>
    {/each}
  {/if}
</div>

<style>
  .gallery {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(9rem, 1fr));
    gap: 0.8rem;
    padding: 1rem;
    overflow-y: auto;
    height: 100%;
    box-sizing: border-box;
    align-content: start;
  }
  .tile {
    margin: 0;
    padding: 0;
    font: inherit;
    text-align: left;
    cursor: pointer;
    background: var(--card-bg);
    border: 1px solid var(--card-border);
    border-radius: 10px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
  .tile:hover {
    border-color: var(--accent);
  }
  .thumb {
    aspect-ratio: 4 / 3;
    display: grid;
    place-items: center;
    background: var(--column-bg);
  }
  .caption {
    padding: 0.45rem 0.55rem;
    font-size: 0.78rem;
    color: var(--text);
    overflow-wrap: anywhere;
  }
  .empty {
    grid-column: 1 / -1;
    padding: 2rem;
    color: var(--muted);
  }
</style>
