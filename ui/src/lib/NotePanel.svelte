<script lang="ts">
  import { getNote } from './ipc';
  import { renderInto } from './render';
  import type { NoteDetail } from './types';

  let { id, onclose }: { id: string; onclose: () => void } = $props();

  let note = $state<NoteDetail | null>(null);
  let content = $state<HTMLElement | undefined>(undefined);
  let error = $state<string | null>(null);

  // Real asset-protocol resolution (asset_status / resolve_asset) lands later;
  // until then every asset renders the inline "not available" placeholder.
  async function resolveAsset(_ref: string): Promise<string | null> {
    return null;
  }

  $effect(() => {
    note = null;
    getNote(id)
      .then((n) => (note = n))
      .catch((e) => (error = String(e)));
  });

  $effect(() => {
    if (note && content) {
      renderInto(content, note.body, resolveAsset).catch((e) => (error = String(e)));
    }
  });
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<div class="overlay">
  <button class="backdrop" aria-label="close note" onclick={onclose}></button>
  <article class="panel">
    <header>
      {#if note}<span class="type" data-type={note.type}>{note.type}</span>{/if}
      <h2>{note?.title ?? 'note'}</h2>
      <button class="close" onclick={onclose} aria-label="close">✕</button>
    </header>
    {#if error}
      <p class="err">{error}</p>
    {/if}
    {#if note}
      <div class="read" bind:this={content}></div>
    {:else if !error}
      <p class="loading">Loading…</p>
    {/if}
  </article>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding: 3rem 1rem;
    z-index: 50;
    overflow-y: auto;
  }
  /* A full-area button behind the panel: clicking outside closes, with no
     stopPropagation and no listeners on non-interactive elements. */
  .backdrop {
    position: fixed;
    inset: 0;
    border: none;
    background: rgb(0 0 0 / 0.5);
    cursor: default;
  }
  .panel {
    position: relative;
    background: var(--panel);
    border: 1px solid var(--column-border);
    border-radius: 12px;
    width: min(46rem, 100%);
    box-shadow: 0 12px 40px rgb(0 0 0 / 0.4);
  }
  header {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.7rem 1rem;
    border-bottom: 1px solid var(--column-border);
  }
  .type {
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--muted);
    border: 1px solid var(--card-border);
    border-radius: 999px;
    padding: 0.05rem 0.5rem;
  }
  h2 {
    flex: 1;
    margin: 0;
    font-size: 1rem;
    color: var(--text);
  }
  .close {
    background: none;
    border: none;
    color: var(--muted);
    font-size: 1rem;
    cursor: pointer;
  }
  .read {
    padding: 1rem 1.4rem 1.6rem;
    line-height: 1.6;
    color: var(--text);
  }
  .loading,
  .err {
    padding: 1.4rem;
    color: var(--muted);
  }
</style>
