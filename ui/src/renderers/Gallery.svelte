<script lang="ts">
  import AssetMissing from '../lib/AssetMissing.svelte';
  import type { ObjectMeta } from '../lib/types';

  // The second renderer. It receives the same ObjectMeta the board does — the
  // only difference from the board is the query behind it (type = asset) and the
  // shape it paints (a grid, not columns). No status literal, no bespoke query.
  let { cards }: { cards: ObjectMeta[] } = $props();
</script>

<div class="gallery">
  {#if cards.length === 0}
    <p class="empty">No assets yet. Media ingest (hashing, thumbnails) arrives with S5.</p>
  {:else}
    {#each cards as card (card.id)}
      <figure class="tile" data-type={card.type}>
        <div class="thumb">
          <!-- No blob resolver yet (S5 adds asset_status / resolve_asset), so
               every asset shows the shared placeholder — graceful absence. -->
          <AssetMissing label={card.title ?? card.preview} />
        </div>
        <figcaption>{card.title ?? card.preview ?? card.id}</figcaption>
      </figure>
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
    background: var(--card-bg);
    border: 1px solid var(--card-border);
    border-radius: 10px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
  .thumb {
    aspect-ratio: 4 / 3;
    display: grid;
    place-items: center;
    background: var(--column-bg);
  }
  figcaption {
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
