<script lang="ts">
  // **A section as a gallery** (the shell, P2a): a tile per note, its first picture when it has one,
  // its name under it. The thumbnail is the small copy when the transport has one (`assetUrl`'s
  // `thumb`), so a page of tiles costs what a page of thumbnails costs.
  import type { ObjectMeta } from '../lib/types';
  import EmptyState from '../lib/EmptyState.svelte';
  import { assetUrl } from '../lib/ipc';

  let { cards, onopen }: { cards: ObjectMeta[]; onopen: (id: string) => void } = $props();
  const pictureOf = (c: ObjectMeta) => (c.assets && c.assets.length ? c.assets[0] : null);
  let broken = $state(new Set<string>());
</script>

{#if !cards.length}
  <EmptyState icon="list" title="Nothing here yet" hint="Press ＋ New to add the first one." />
{:else}
  <ul class="gallery" aria-label="gallery">
    {#each cards as c (c.id)}
      {@const pic = pictureOf(c)}
      <li>
        <button type="button" class="tile" onclick={() => onopen(c.id)}>
          <span class="pic" class:none={!pic || broken.has(pic)}>
            {#if pic && !broken.has(pic)}
              <img
                src={assetUrl(pic, 'thumb')}
                alt=""
                loading="lazy"
                decoding="async"
                onerror={() => (broken = new Set(broken).add(pic))}
              />
            {:else}
              <span aria-hidden="true">{(c.title || c.preview || '?').slice(0, 1)}</span>
            {/if}
          </span>
          <span class="name">{c.title || c.preview || 'Untitled'}</span>
        </button>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .gallery {
    list-style: none;
    margin: 0;
    padding: var(--space-3) var(--space-4);
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(10rem, 1fr));
    gap: var(--space-3);
  }
  .tile {
    width: 100%;
    display: flex;
    flex-direction: column;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    overflow: hidden;
    cursor: pointer;
    text-align: left;
    font: inherit;
    color: inherit;
  }
  .tile:hover {
    border-color: var(--accent);
  }
  .pic {
    aspect-ratio: 4 / 3;
    display: grid;
    place-items: center;
    background: var(--card-bg);
    color: var(--text-muted);
    font-size: 2rem;
  }
  .pic img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .name {
    padding: var(--space-2) var(--space-3);
    font-size: var(--text-sm);
    font-weight: 600;
  }
</style>
