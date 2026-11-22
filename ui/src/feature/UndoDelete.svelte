<script lang="ts">
  // **Undo, on every page of formicaria's browser** (`ui-audit-2026-11-22.md`, U1). Measured: a note
  // deleted on a computer offered no Undo, and Ctrl+Z did not bring it back. A delete is said here
  // with *Undo*, on whichever tab the person is on (the note's own tab closes, so the next tab in
  // front hears of it through the browser's storage, `lastDeleted.ts`). Ctrl+Z / ⌘Z outside the text
  // undoes this page's last action, or else puts back the last note deleted in the last half hour.
  import { onMount } from 'svelte';
  import { restoreNote } from '../lib/ipc';
  import * as undo from '../lib/undo.svelte';
  import {
    DELETED_EVENT,
    DELETED_KEY,
    forgetDeleted,
    recentDeleted,
    type LastDeleted,
  } from '../lib/lastDeleted';

  let { onrestored = () => {} }: { onrestored?: (id: string) => void } = $props();

  let shown = $state<LastDeleted | null>(null);
  let said = $state('');
  let timer: ReturnType<typeof setTimeout> | undefined;

  function offer(d: LastDeleted | null) {
    if (!d) return;
    shown = d;
    said = '';
    clearTimeout(timer);
    timer = setTimeout(() => (shown = null), 10000);
  }

  async function putBack(d: LastDeleted) {
    try {
      await restoreNote(d.vault, d.file);
      forgetDeleted(d.id);
      shown = null;
      say(`Put back “${d.title}”`);
      onrestored(d.id);
    } catch (e) {
      say(`Could not put it back: ${String(e)}`);
    }
  }

  function say(text: string) {
    said = text;
    clearTimeout(timer);
    timer = setTimeout(() => (said = ''), 5000);
  }

  const typing = (t: EventTarget | null) => {
    const el = t as HTMLElement | null;
    return (
      !!el &&
      (el.isContentEditable ||
        /^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName) ||
        !!el.closest?.('.cm-editor, .excalidraw'))
    );
  };

  async function onKey(e: KeyboardEvent) {
    if (!(e.ctrlKey || e.metaKey) || e.shiftKey || e.altKey || e.key.toLowerCase() !== 'z') return;
    if (typing(e.target)) return; // the text keeps its own undo
    e.preventDefault();
    if (undo.nextUndo()) {
      const label = await undo.undo();
      if (label) say(`Undid ${label}`);
      return;
    }
    const d = recentDeleted(30 * 60 * 1000);
    if (d) await putBack(d);
    else say('Nothing to undo');
  }

  onMount(() => {
    // Deleted in this tab, or a moment ago in the tab that just closed.
    offer(recentDeleted(15000));
    const onStorage = (e: StorageEvent) => {
      if (e.key === DELETED_KEY) {
        if (e.newValue) offer(recentDeleted(15000));
        else shown = null;
      }
    };
    // Deleted in this very page; or Back brought a page out of the browser's cache, which mounts
    // nothing and hears no storage event of its own making.
    const here = () => offer(recentDeleted(15000));
    const shownAgain = (e: PageTransitionEvent) => e.persisted && here();
    addEventListener('storage', onStorage);
    addEventListener('keydown', onKey);
    addEventListener(DELETED_EVENT, here);
    addEventListener('pageshow', shownAgain);
    return () => {
      removeEventListener('storage', onStorage);
      removeEventListener('keydown', onKey);
      removeEventListener(DELETED_EVENT, here);
      removeEventListener('pageshow', shownAgain);
      clearTimeout(timer);
    };
  });
</script>

{#if shown}
  <div class="toast" role="status">
    <span>Deleted “{shown.title}”</span>
    <button type="button" onclick={() => shown && void putBack(shown)}>Undo</button>
    <button type="button" class="x" aria-label="dismiss" onclick={() => (shown = null)}>✕</button>
  </div>
{:else if said}
  <div class="toast" role="status"><span>{said}</span></div>
{/if}

<style>
  .toast {
    position: fixed;
    left: 50%;
    bottom: var(--space-5);
    translate: -50% 0;
    z-index: 300;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3) var(--space-2) var(--space-4);
    background: var(--text);
    color: var(--bg);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
    font-size: var(--text-sm);
    max-width: calc(100vw - 2 * var(--space-4));
  }
  .toast button {
    font: inherit;
    font-weight: 600;
    color: inherit;
    background: transparent;
    border: 1px solid color-mix(in srgb, var(--bg) 40%, transparent);
    border-radius: var(--radius-md);
    padding: var(--space-1) var(--space-3);
    cursor: pointer;
  }
  .toast .x {
    border: none;
    padding: var(--space-1);
    opacity: 0.7;
  }
</style>
