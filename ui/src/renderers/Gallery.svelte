<script lang="ts">
  import AssetMissing from '../lib/AssetMissing.svelte';
  import { resolveAsset } from '../lib/ipc';
  import type { ObjectMeta } from '../lib/types';

  // The second renderer. It receives the same ObjectMeta the board does — the
  // only difference from the board is the query behind it (type = asset) and the
  // shape it paints (a grid, not columns). No status literal, no bespoke query.
  let { cards, onopen }: { cards: ObjectMeta[]; onopen: (id: string) => void } = $props();

  // Load each tile's derived thumbnail as an object URL. A card with no asset, a
  // missing blob, or the browser/test mock resolves to null and shows the shared
  // AssetMissing placeholder — absence is a warning, never an error. URLs are
  // revoked when the card set changes or the view unmounts.
  let thumbs = $state<Record<string, string | null>>({});
  $effect(() => {
    const created: string[] = [];
    let cancelled = false;
    (async () => {
      for (const card of cards) {
        const ref = card.assets?.[0];
        if (!ref) {
          if (!cancelled) thumbs[card.id] = null;
          continue;
        }
        try {
          const buf = await resolveAsset(ref, 'thumb');
          if (cancelled) return;
          if (buf && buf.byteLength > 0) {
            const url = URL.createObjectURL(new Blob([buf]));
            created.push(url);
            thumbs[card.id] = url;
          } else {
            thumbs[card.id] = null;
          }
        } catch {
          if (!cancelled) thumbs[card.id] = null;
        }
      }
    })();
    return () => {
      cancelled = true;
      for (const u of created) URL.revokeObjectURL(u);
    };
  });
</script>

<div class="gallery">
  {#if cards.length === 0}
    <p class="empty">No assets yet. Ingest media with `fm add` (hashing, thumbnails).</p>
  {:else}
    {#each cards as card (card.id)}
      <button class="tile" data-type={card.type} onclick={() => onopen(card.id)}>
        <span class="thumb">
          {#if thumbs[card.id]}
            <img src={thumbs[card.id]} alt={card.title ?? card.preview ?? ''} loading="lazy" />
          {:else}
            <!-- No asset, missing blob, or no thumbnail yet: graceful absence. -->
            <AssetMissing label={card.title ?? card.preview} />
          {/if}
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
    border-radius: var(--radius-md);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    transition:
      border-color var(--dur-fast) var(--ease),
      box-shadow var(--dur-fast) var(--ease),
      transform var(--dur-fast) var(--ease);
  }
  .tile:hover {
    border-color: var(--accent);
    box-shadow: var(--shadow-md);
    transform: translateY(-2px);
  }
  .thumb {
    aspect-ratio: 4 / 3;
    display: grid;
    place-items: center;
    background: var(--column-bg);
    overflow: hidden;
  }
  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
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
