<script lang="ts">
  // **＋ New, from the rail** (the shell, P2b; the parity list: the older frame's ＋ made a board, a
  // discussion, a paper and a note from a template, which the computer's pages could not). The same
  // makes, the same calls (`App.svelte`'s `onNewBoard`, `onNewDiscussion`, `confirmNewPaper`,
  // `onNewFromTemplate`), each opening what it made.
  import * as ipc from '../lib/ipc';
  import { draftFrom } from '../lib/templates';
  import { createNotesUndoable } from '../lib/undo.svelte';
  import { popup } from '../lib/popup';
  import type { ObjectMeta } from '../lib/types';

  let { onopen }: { onopen: (id: string) => void } = $props();

  let open = $state(false);
  let templates = $state<ObjectMeta[]>([]);
  let paper = $state<string | null>(null);
  let failed = $state('');
  let btn = $state<HTMLButtonElement | undefined>(undefined);

  async function show() {
    open = !open;
    if (open) templates = await ipc.tagged('template').catch(() => []);
  }
  async function run(make: () => Promise<string | undefined>) {
    open = false;
    failed = '';
    try {
      const id = await make();
      if (id) onopen(id);
    } catch (e) {
      failed = String(e);
    }
  }
  const note = async () => {
    const [m] = await ipc.createNotes([{ title: '', body: '', tags: [], start: '', props: {} }]);
    return m?.id;
  };
  const board = async () => {
    const scene =
      '{"type":"excalidraw","version":2,"source":"formicaria","elements":[],"appState":{},"files":{}}';
    const meta = await ipc.capture(scene, '');
    await ipc.setProperty(meta.id, 'view', 'board');
    await ipc.setProperty(meta.id, 'title', 'Untitled board');
    return meta.id;
  };
  const discussion = async () => (await ipc.createDiscussion('Untitled discussion', '')).id;
  const fromTemplate = (t: ObjectMeta) => async () => {
    const tpl = await ipc.getNote(t.id);
    if (!tpl) throw new Error('that template is no longer there');
    const [m] = await createNotesUndoable(
      [draftFrom(tpl)],
      `made a note from “${tpl.title || 'a template'}”`,
      '',
    );
    return m?.id;
  };
  async function addPaper() {
    const what = (paper ?? '').trim();
    if (!what) return;
    paper = null;
    await run(async () => (await ipc.createPaper(what, '')).id);
  }
</script>

<div class="new">
  <button
    type="button"
    class="plus"
    bind:this={btn}
    aria-haspopup="menu"
    aria-expanded={open}
    onclick={() => void show()}>＋ New</button
  >
  {#if open}
    <ul
      class="menu"
      role="menu"
      aria-label="make something new"
      use:popup={{ anchor: btn, align: 'start' }}
    >
      <li role="none">
        <button type="button" role="menuitem" onclick={() => void run(note)}>New note</button>
      </li>
      <li role="none">
        <button type="button" role="menuitem" onclick={() => void run(board)}>New board</button>
      </li>
      <li role="none">
        <button type="button" role="menuitem" onclick={() => void run(discussion)}
          >New discussion</button
        >
      </li>
      <li role="none">
        <button type="button" role="menuitem" onclick={() => ((open = false), (paper = ''))}
          >New paper…</button
        >
      </li>
      {#each templates as t (t.id)}
        <li role="none">
          <button type="button" role="menuitem" onclick={() => void run(fromTemplate(t))}
            >New from “{t.title || t.preview || 'Untitled'}”</button
          >
        </li>
      {/each}
    </ul>
  {/if}
  {#if paper !== null}
    <form class="paper" onsubmit={(e) => (e.preventDefault(), void addPaper())}>
      <input
        bind:value={paper}
        placeholder="A DOI, an arXiv link, or a BibTeX entry"
        aria-label="the paper to add"
      />
      <button type="submit">Add</button>
      <button type="button" onclick={() => (paper = null)}>Cancel</button>
    </form>
  {/if}
  {#if failed}<p class="error" role="alert">{failed}</p>{/if}
</div>

<style>
  .plus {
    width: 100%;
    font: inherit;
    font-weight: 600;
    padding: var(--space-1) var(--space-2);
    border: 1px solid var(--accent);
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--accent-fg, #fff);
    cursor: pointer;
  }
  .menu {
    list-style: none;
    margin: 0;
    padding: var(--space-1);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-lg);
    min-width: 14rem;
    z-index: 50;
  }
  .menu button {
    width: 100%;
    text-align: left;
    border: none;
    background: none;
    padding: var(--space-1) var(--space-2);
    font: inherit;
    cursor: pointer;
    border-radius: var(--radius-sm);
  }
  .menu button:hover {
    background: var(--card-bg);
  }
  .paper {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin-top: var(--space-1);
  }
  .paper input {
    font: inherit;
    padding: var(--space-1);
  }
  .error {
    color: var(--danger-fg);
    font-size: var(--text-xs);
  }
</style>
