<script lang="ts">
  // **Select, then bring it somewhere** (`decisions.md` 2026-11-17, *one frame*; the owner: *"a small
  // icon floating close to the selection"*). Anywhere inside formicaria (a note, a feature), a
  // selection shows formicaria's icon beside it; pressed, it asks where: the note looked at last, any
  // note found by typing, or a new note. The passage goes in by `put`, quoted, with where it came
  // from. On Android the system's selection bar has *Bring into note* first too (`MainActivity`),
  // which opens the same chooser (`fm-bring-selection`).
  import { onMount } from 'svelte';
  import { put, search } from './ipc';
  import { fromNote } from './piece';
  import { appToday, clockText } from './appTime';
  import { lastNote } from './lastNote';

  let {
    source,
  }: {
    /** The note the selection is in, when it is in one (for the link back). */
    source: () => { id: string; title: string; body?: string } | null;
  } = $props();

  let at = $state<{ x: number; y: number } | null>(null);
  let text = $state('');
  let choosing = $state(false);
  let q = $state('');
  let hits = $state<{ id: string; title: string }[]>([]);
  let said = $state('');
  let timer: ReturnType<typeof setTimeout> | undefined;

  const skip = (n: Node | null) => {
    const e = n instanceof Element ? n : n?.parentElement;
    return !!e?.closest(
      'input, textarea, [contenteditable=""], [contenteditable="true"], .fm-bring',
    );
  };
  function look() {
    if (choosing) return;
    const sel = getSelection();
    const t = sel && !sel.isCollapsed ? sel.toString().trim() : '';
    if (!t || skip(sel!.anchorNode)) {
      at = null;
      return;
    }
    const r = sel!.getRangeAt(sel!.rangeCount - 1);
    const rects = r.getClientRects();
    const last = rects[rects.length - 1] ?? r.getBoundingClientRect();
    text = t;
    at = {
      x: Math.min(Math.max(4, last.right + 6), innerWidth - 40),
      y: Math.min(Math.max(4, last.bottom + 6), innerHeight - 40),
    };
  }
  function soon() {
    clearTimeout(timer);
    timer = setTimeout(look, 250);
  }
  // A look still waiting when this goes (the page left, a test torn down) is not taken: it read a
  // selection from a document that was no longer there (2026-11-20, an error after a test ended).
  $effect(() => () => clearTimeout(timer));

  const here = () => source();
  /** The note it goes to by default: the one looked at last, never the one it comes from. */
  const into = $derived.by(() => {
    void choosing;
    const l = lastNote();
    return l && l.id !== here()?.id ? l : null;
  });

  function piece(): string {
    const s = here();
    const when = `${appToday()} ${clockText()}`;
    if (s) return fromNote({ id: s.id, title: s.title, body: s.body ?? '' }, text, when).md;
    return text
      .split('\n')
      .map((l) => (l.trim() ? `> ${l}` : '>'))
      .join('\n');
  }
  async function bring(to: { note: string; title: string } | { fresh: true }) {
    try {
      if ('fresh' in to) {
        await put(
          { new: { title: here()?.title ? `From ${here()!.title}` : 'A passage' } },
          piece(),
        );
        said = 'Brought into a new note';
      } else {
        await put({ note: to.note }, piece());
        said = `Brought into “${to.title}”`;
      }
    } catch (e) {
      said = String(e);
    }
    choosing = false;
    at = null;
    getSelection()?.removeAllRanges();
    setTimeout(() => (said = ''), 3000);
  }
  async function find() {
    const t = q.trim();
    if (!t) return (hits = []);
    try {
      hits = (await search(t)).slice(0, 6).map((n) => ({ id: n.id, title: n.title || 'Untitled' }));
    } catch {
      hits = [];
    }
  }

  onMount(() => {
    const open = () => {
      look();
      if (text) choosing = true;
    };
    document.addEventListener('selectionchange', soon);
    addEventListener('fm-bring-selection', open);
    return () => {
      document.removeEventListener('selectionchange', soon);
      removeEventListener('fm-bring-selection', open);
    };
  });
</script>

{#if at && !choosing}
  <button
    type="button"
    class="fm-bring icon"
    style:left="{at.x}px"
    style:top="{at.y}px"
    aria-label="bring the selection into a note"
    title="Bring into a note"
    onpointerdown={(e) => e.preventDefault()}
    onclick={() => (choosing = true)}
  >
    <img src="/favicon.svg" alt="" width="22" height="22" />
  </button>
{/if}
{#if choosing}
  <div
    class="fm-bring chooser"
    role="dialog"
    aria-label="where it goes"
    style:left="{Math.min(at?.x ?? 8, innerWidth - 290)}px"
    style:top="{Math.min(at?.y ?? 8, innerHeight - 260)}px"
  >
    {#if into}
      <button
        type="button"
        class="main"
        onclick={() => void bring({ note: into.id, title: into.title })}>Into “{into.title}”</button
      >
    {/if}
    <input
      type="search"
      placeholder="Another note…"
      aria-label="find the note to bring it into"
      bind:value={q}
      oninput={() => void find()}
      onkeydown={(e) => {
        if (e.key === 'Enter' && hits[0]) void bring({ note: hits[0].id, title: hits[0].title });
        if (e.key === 'Escape') choosing = false;
      }}
    />
    {#each hits as h (h.id)}
      <button type="button" onclick={() => void bring({ note: h.id, title: h.title })}
        >{h.title}</button
      >
    {/each}
    <button type="button" onclick={() => void bring({ fresh: true })}>Into a new note</button>
    <button type="button" class="cancel" onclick={() => ((choosing = false), (at = null))}
      >Cancel</button
    >
  </div>
{/if}
{#if said}<p class="fm-bring said" role="status">{said}</p>{/if}

<style>
  .fm-bring {
    position: fixed;
    z-index: 60;
  }
  .icon {
    width: 36px;
    height: 36px;
    padding: 6px;
    border-radius: 10px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
    background: #dc2626;
  }
  .icon img {
    display: block;
    filter: brightness(0) invert(1);
  }
  .chooser {
    width: 280px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px;
    border-radius: 10px;
    background: var(--bg, Canvas);
    box-shadow: 0 6px 24px rgba(0, 0, 0, 0.28);
  }
  .chooser button {
    text-align: start;
    padding: 7px 9px;
    border-radius: 6px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .chooser .main {
    font-weight: 600;
  }
  .chooser input {
    margin: 4px 0;
    padding: 7px 9px;
  }
  .cancel {
    opacity: 0.7;
  }
  .said {
    left: 50%;
    transform: translateX(-50%);
    bottom: calc(var(--safe-bottom, 0px) + 5rem);
    margin: 0;
    padding: 8px 14px;
    border-radius: 999px;
    background: var(--bg, Canvas);
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.25);
  }
</style>
