<script lang="ts">
  import Board from './renderers/Board.svelte';
  import Gallery from './renderers/Gallery.svelte';
  import Agenda from './renderers/Agenda.svelte';
  import { getBoard, getGallery, getAgenda, capture, setProperty } from './lib/ipc';
  import type { Board as BoardData, ObjectMeta } from './lib/types';

  type View = 'board' | 'gallery' | 'agenda';
  let view = $state<View>('board');
  let groupBy = $state('status');
  let board = $state<BoardData | null>(null);
  let cards = $state<ObjectMeta[] | null>(null);
  let draft = $state('');
  let error = $state<string | null>(null);
  let openId = $state<string | null>(null);

  async function refresh() {
    try {
      if (view === 'board') board = await getBoard(groupBy);
      else if (view === 'gallery') cards = await getGallery();
      else cards = await getAgenda();
      error = null;
    } catch (e) {
      error = String(e);
    }
  }

  // Reload when the view switches, or (in board view) when the grouping changes.
  $effect(() => {
    void view;
    if (view === 'board') void groupBy;
    void refresh();
  });

  async function onCapture(e: Event) {
    e.preventDefault();
    const body = draft.trim();
    if (!body) return;
    draft = '';
    try {
      await capture(body);
      await refresh();
    } catch (err) {
      error = String(err);
    }
  }

  async function onMove(id: string, value: string) {
    // The drag write-back: set the grouped property to the target column's value.
    try {
      await setProperty(id, groupBy, value);
      await refresh();
    } catch (err) {
      error = String(err);
    }
  }
</script>

<main>
  <header class="topbar">
    <h1>formicarium</h1>
    <form class="capture" onsubmit={onCapture}>
      <!-- svelte-ignore a11y_autofocus -->
      <input placeholder="Capture a thought…" bind:value={draft} autofocus />
    </form>
    <div class="views" role="group" aria-label="view">
      <button class:active={view === 'board'} aria-pressed={view === 'board'} onclick={() => (view = 'board')}>
        Board
      </button>
      <button class:active={view === 'agenda'} aria-pressed={view === 'agenda'} onclick={() => (view = 'agenda')}>
        Agenda
      </button>
      <button class:active={view === 'gallery'} aria-pressed={view === 'gallery'} onclick={() => (view = 'gallery')}>
        Gallery
      </button>
    </div>
    {#if view === 'board'}
      <label class="group">
        <span>group by</span>
        <input list="props" bind:value={groupBy} spellcheck="false" />
        <datalist id="props">
          <option value="status"></option>
          <option value="type"></option>
          <option value="project"></option>
          <option value="tags"></option>
        </datalist>
      </label>
    {/if}
  </header>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <div class="stage">
    {#if view === 'board'}
      {#if board}
        <Board {board} onmove={onMove} onopen={(id) => (openId = id)} />
      {:else}
        <p class="empty">Loading…</p>
      {/if}
    {:else if !cards}
      <p class="empty">Loading…</p>
    {:else if view === 'gallery'}
      <Gallery {cards} onopen={(id) => (openId = id)} />
    {:else}
      <Agenda {cards} onopen={(id) => (openId = id)} />
    {/if}
  </div>

  {#if openId}
    <!-- Lazy: the read view (marked + KaTeX + Mermaid) only loads when a note is
         opened, so the core bundle stays small. -->
    {#await import('./lib/NotePanel.svelte') then { default: NotePanel }}
      <NotePanel id={openId} onclose={() => (openId = null)} />
    {/await}
  {/if}
</main>

<style>
  main {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }
  .topbar {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.6rem 1rem;
    border-bottom: 1px solid var(--column-border);
    background: var(--panel);
  }
  h1 {
    margin: 0;
    font-size: 1rem;
    font-weight: 700;
    letter-spacing: 0.02em;
    color: var(--accent);
  }
  .capture {
    flex: 1;
  }
  .capture input {
    width: 100%;
    padding: 0.45rem 0.7rem;
    border-radius: 8px;
    border: 1px solid var(--card-border);
    background: var(--card-bg);
    color: var(--text);
    font-size: 0.9rem;
  }
  .views {
    display: flex;
    border: 1px solid var(--card-border);
    border-radius: 8px;
    overflow: hidden;
  }
  .views button {
    padding: 0.4rem 0.7rem;
    background: var(--card-bg);
    color: var(--muted);
    border: none;
    cursor: pointer;
    font-size: 0.82rem;
  }
  .views button.active {
    background: var(--column-over-bg);
    color: var(--text);
  }
  .group {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.8rem;
    color: var(--muted);
  }
  .group input {
    width: 7rem;
    padding: 0.35rem 0.5rem;
    border-radius: 7px;
    border: 1px solid var(--card-border);
    background: var(--card-bg);
    color: var(--text);
  }
  .stage {
    flex: 1;
    min-height: 0;
  }
  .error {
    margin: 0;
    padding: 0.5rem 1rem;
    background: #5a2330;
    color: #ffd7dd;
    font-size: 0.85rem;
  }
  .empty {
    padding: 2rem;
    color: var(--muted);
  }
</style>
