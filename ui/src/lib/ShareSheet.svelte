<script lang="ts">
  // **Where a shared picture goes** (`decisions.md#ui`, 2026-10-20). Android's Share button hands
  // formicaria a screenshot; this asks where it belongs, the way note apps on a phone do: Keep
  // makes a new note, Notion and Obsidian offer "add to" a page found by name. Here the note open
  // now comes first, since a screenshot taken while writing one is most often for it; then a new
  // note; then any other, from the recent ones or by name. Nothing is written until one is chosen.
  import { onDestroy, onMount } from 'svelte';
  import { getNote, recent, search } from './ipc';
  import { noteName } from './noteName';
  import type { ObjectMeta } from './types';
  import type { Refused } from './share.svelte';

  let {
    files,
    refused = [],
    openNoteId = null,
    onchoose,
    onclose,
  }: {
    files: File[];
    /** What the phone could not take, and why. */
    refused?: Refused[];
    /** The note in front, if one is: offered first. */
    openNoteId?: string | null;
    /** `null` is a new note. */
    onchoose: (noteId: string | null) => void;
    onclose: () => void;
  } = $props();

  const what = $derived.by(() => {
    const pictures = files.every((f) => f.type.startsWith('image/'));
    const noun = pictures ? 'picture' : 'file';
    return files.length === 1 ? `the ${noun}` : `${files.length} ${noun}s`;
  });
  const previews = $derived(
    files.filter((f) => f.type.startsWith('image/')).map((f) => URL.createObjectURL(f)),
  );
  onDestroy(() => previews.forEach((u) => URL.revokeObjectURL(u)));

  let openTitle = $state<string | null>(null);
  let query = $state('');
  let rows = $state<ObjectMeta[]>([]);
  let timer: ReturnType<typeof setTimeout> | undefined;

  onMount(async () => {
    if (openNoteId) {
      const n = await getNote(openNoteId).catch(() => null);
      openTitle = n ? (noteName(n) ?? 'Untitled note') : null;
    }
    await find();
  });

  /** Recent notes while nothing is typed; notes found by name once something is. Boards and
   *  discussions are left out: a picture cannot be put in either from here. */
  async function find() {
    const q = query.trim();
    try {
      const found = q ? await search(q) : await recent();
      rows = found
        .filter(
          (n) =>
            n.type === 'note' &&
            n.id !== openNoteId &&
            n.props?.view !== 'board' &&
            !n.props?.thread_of,
        )
        .slice(0, 8);
    } catch {
      rows = [];
    }
  }
  function typed() {
    clearTimeout(timer);
    timer = setTimeout(find, 150);
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose();
  }
</script>

<svelte:window {onkeydown} />

<div class="share-overlay">
  <button class="share-backdrop" aria-label="close" onclick={onclose}></button>
  <div class="panel" role="dialog" aria-label="add what was shared to a note">
    <div class="panel-head">
      <h2>{files.length ? `Add ${what} to…` : 'Nothing could be added'}</h2>
      <button type="button" class="close" aria-label="close" onclick={onclose}>✕</button>
    </div>
    {#if previews.length}
      <div class="previews">
        {#each previews as src (src)}<img {src} alt="" />{/each}
      </div>
    {/if}
    {#if refused.length}
      <p class="failed" role="alert">
        Not added: {refused.map((r) => `${r.name} (${r.why})`).join(', ')}.
        {#if refused.some((r) => r.why === 'too large')}Open the note and add it with 📎.{/if}
        {#if refused.some((r) => r.why !== 'too large')}Try sharing it again, or add it with 📎 from
          the note.{/if}
      </p>
    {/if}
    {#if files.length}
      <div class="choices">
        {#if openNoteId && openTitle}
          <button type="button" class="choice first" onclick={() => onchoose(openNoteId)}>
            <span class="title">{openTitle}</span>
            <span class="meta">the note open now</span>
          </button>
        {/if}
        <button type="button" class="choice" onclick={() => onchoose(null)}>
          <span class="title">New note</span>
        </button>
      </div>
      <input
        class="find"
        type="search"
        placeholder="Find a note"
        aria-label="find a note"
        bind:value={query}
        oninput={typed}
      />
      <ul class="list" aria-label={query.trim() ? 'notes found' : 'recent notes'}>
        {#each rows as n (n.id)}
          <li>
            <button type="button" class="choice" onclick={() => onchoose(n.id)}>
              <span class="title">{noteName(n) ?? 'Untitled note'}</span>
            </button>
          </li>
        {:else}
          {#if query.trim()}<li class="meta">No note by that name.</li>{/if}
        {/each}
      </ul>
    {/if}
    <button type="button" class="cancel" onclick={onclose}
      >{files.length ? 'Cancel' : 'Close'}</button
    >
  </div>
</div>

<style>
  .share-overlay {
    position: fixed;
    inset: 0;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    box-sizing: border-box;
    height: var(--app-h);
    padding-top: calc(var(--overlay-inset) + var(--safe-top));
    padding-bottom: var(--safe-bottom);
    z-index: 80;
  }
  @media (pointer: coarse) {
    .share-overlay {
      padding-bottom: max(var(--safe-bottom), var(--bar-floor));
    }
  }
  .share-backdrop {
    position: fixed;
    inset: 0;
    border: none;
    background: rgb(0 0 0 / 0.45);
    cursor: default;
  }
  .panel {
    position: relative;
    width: min(30rem, 92vw);
    max-height: 100%;
    overflow-y: auto;
    overscroll-behavior: contain;
    box-sizing: border-box;
    padding: var(--space-5);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    background: var(--surface-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
  }
  .panel-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
  }
  h2 {
    margin: 0;
    font-size: var(--text-md);
    color: var(--text);
  }
  .close,
  .cancel {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 1rem;
  }
  .cancel {
    align-self: flex-end;
    font-size: var(--text-sm);
    padding: var(--space-2);
  }
  .previews {
    display: flex;
    gap: var(--space-2);
    overflow-x: auto;
  }
  .previews img {
    height: 5rem;
    max-width: 10rem;
    object-fit: cover;
    border-radius: var(--radius-sm, 4px);
    border: 1px solid var(--border);
  }
  .choices {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  /* Big enough to hit with a thumb: this is a phone screen first. */
  .choice {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    width: 100%;
    min-height: 44px;
    padding: var(--space-2) var(--space-3);
    box-sizing: border-box;
    text-align: left;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md, 6px);
    color: var(--text);
    cursor: pointer;
  }
  .choice.first {
    border-color: var(--accent);
  }
  .title {
    font-weight: 600;
    overflow-wrap: anywhere;
  }
  .meta {
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .find {
    min-height: 44px;
    padding: 0 var(--space-3);
    box-sizing: border-box;
    border: 1px solid var(--border);
    border-radius: var(--radius-md, 6px);
    background: var(--surface);
    color: var(--text);
    font-size: 16px; /* below 16px a phone zooms in on focus */
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .failed {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--danger, #b00);
  }
</style>
