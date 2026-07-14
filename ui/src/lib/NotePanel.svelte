<script lang="ts">
  import { getNote, updateBody } from './ipc';
  import { renderInto } from './render';
  import type { NoteDetail } from './types';

  let { id, onclose }: { id: string; onclose: () => void } = $props();

  let note = $state<NoteDetail | null>(null);
  let content = $state<HTMLElement | undefined>(undefined);
  let error = $state<string | null>(null);
  let editing = $state(false);
  let draft = $state('');
  let saved = $state(true);
  let saveTimer: ReturnType<typeof setTimeout> | undefined;

  // Real asset-protocol resolution (asset_status / resolve_asset) lands later;
  // until then every asset renders the inline "not available" placeholder.
  async function resolveAsset(_ref: string): Promise<string | null> {
    return null;
  }

  $effect(() => {
    note = null;
    editing = false;
    getNote(id)
      .then((n) => {
        note = n;
        draft = n?.body ?? '';
      })
      .catch((e) => (error = String(e)));
  });

  // Render the read view when not editing (a textarea holds the literal bytes).
  $effect(() => {
    if (note && content && !editing) {
      renderInto(content, note.body, resolveAsset).catch((e) => (error = String(e)));
    }
  });

  // Debounced save: typing stops -> 500 ms -> atomic write via update_body.
  function onInput() {
    saved = false;
    clearTimeout(saveTimer);
    saveTimer = setTimeout(save, 500);
  }

  async function save() {
    if (!note) return;
    try {
      await updateBody(note.id, draft);
      note = { ...note, body: draft };
      saved = true;
    } catch (e) {
      error = String(e);
    }
  }

  async function toggleEdit() {
    if (editing) await save(); // leaving edit mode flushes any pending change
    editing = !editing;
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<div class="overlay">
  <button class="backdrop" aria-label="close note" onclick={onclose}></button>
  <article class="panel">
    <header>
      {#if note}<span class="type" data-type={note.type}>{note.type}</span>{/if}
      <h2>{note?.title ?? 'note'}</h2>
      {#if note}
        <button class="edit" onclick={toggleEdit}>
          {editing ? (saved ? 'Done' : 'Saving…') : 'Edit'}
        </button>
      {/if}
      <button class="close" onclick={onclose} aria-label="close">✕</button>
    </header>
    {#if error}
      <p class="err">{error}</p>
    {/if}
    {#if note}
      {#if editing}
        <textarea
          class="editor"
          bind:value={draft}
          oninput={onInput}
          spellcheck="false"
          aria-label="note body (Markdown)"
        ></textarea>
      {:else}
        <div class="read" bind:this={content}></div>
      {/if}
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
  .edit {
    background: var(--card-bg);
    border: 1px solid var(--card-border);
    border-radius: 7px;
    color: var(--text);
    font-size: 0.78rem;
    padding: 0.25rem 0.6rem;
    cursor: pointer;
  }
  .close {
    background: none;
    border: none;
    color: var(--muted);
    font-size: 1rem;
    cursor: pointer;
  }
  .editor {
    width: 100%;
    min-height: 22rem;
    resize: vertical;
    box-sizing: border-box;
    padding: 1rem 1.4rem;
    border: none;
    background: transparent;
    color: var(--text);
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.85rem;
    line-height: 1.6;
  }
  .editor:focus {
    outline: none;
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
