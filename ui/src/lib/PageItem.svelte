<script lang="ts">
  // **A web page, an item of the collection like a note** (`docs/context/workspace-design-2026-11-15.md`
  // §13.7; `decisions.md` 2026-11-16, *a page is a layer*). On the phone the Android shell draws the
  // page itself, exactly over this box (`__fmPage`, `ci/android-inject-service.sh`); the app only says
  // where, and hides it whenever this box is out of sight (another item in front, a sheet over it).
  // Elsewhere (a computer, where formicaria's browser has real tabs) it is the address, to open.
  import { onDestroy } from 'svelte';
  import type { Pane } from './panes';

  interface PageShell {
    show(id: string, url: string, x: number, y: number, w: number, h: number): void;
    hide(id: string): void;
    back(id: string): void;
    bring(id: string, note: string): void;
  }
  const pageShell = (): PageShell | null =>
    (globalThis as { __fmPage?: PageShell }).__fmPage ?? null;

  let {
    pane,
    target,
  }: {
    pane: Pane;
    /** The note a brought passage goes into (the note before this page), if any. */
    target: () => { id: string; title: string } | null;
  } = $props();

  const shell = pageShell();
  let box = $state<HTMLDivElement>();
  let shown = false;
  let at = '';

  function place() {
    if (!shell || !box || !pane.url) return;
    const r = box.getBoundingClientRect();
    // Out of sight (display:none, or under a sheet or menu the app put up): the page goes too, since
    // the shell draws it above the app.
    const covered = !!document.querySelector('[role=dialog], [role=menu]');
    if (r.width < 4 || r.height < 4 || covered) {
      if (shown) shell.hide(pane.id);
      shown = false;
      at = '';
      return;
    }
    const key = `${r.left}|${r.top}|${r.width}|${r.height}`;
    if (shown && key === at) return;
    shell.show(pane.id, pane.url, r.left, r.top, r.width, r.height);
    shown = true;
    at = key;
  }

  $effect(() => {
    if (!box || !shell) return;
    const ro = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(place);
    ro?.observe(box);
    // A dialog opening or closing anywhere decides whether the page may stay on top.
    const mo = new MutationObserver(place);
    mo.observe(document.body, { childList: true, subtree: true });
    window.addEventListener('resize', place);
    window.addEventListener('scroll', place, true);
    place();
    return () => {
      ro?.disconnect();
      mo.disconnect();
      window.removeEventListener('resize', place);
      window.removeEventListener('scroll', place, true);
    };
  });
  onDestroy(() => {
    if (shown) shell?.hide(pane.id);
  });

  const host = $derived.by(() => {
    try {
      return new URL(pane.url ?? '').host.replace(/^www\./, '');
    } catch {
      return pane.url ?? '';
    }
  });
</script>

<div class="page-item">
  {#if shell}
    <div class="page-head">
      <button type="button" aria-label="back in this page" onclick={() => shell.back(pane.id)}
        >‹</button
      >
      <span class="page-title">{pane.pageTitle || host}</span>
      <button
        type="button"
        aria-label="bring the selection into a note"
        title="Bring the selection (or this page) into your note"
        onclick={() => shell.bring(pane.id, target()?.id ?? '')}>⇥</button
      >
    </div>
    <div class="page-box" bind:this={box}></div>
  {:else}
    <div class="page-link">
      <p>{pane.pageTitle || host}</p>
      <a href={pane.url} target="_blank" rel="noopener">{pane.url}</a>
    </div>
  {/if}
</div>

<style>
  .page-item {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }
  .page-head {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    padding: 0 var(--space-1);
    min-height: 2.4rem;
  }
  .page-head button {
    flex: none;
    min-width: 2.4rem;
    min-height: 2.4rem;
    font-size: 1.1rem;
  }
  .page-title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--text-sm);
  }
  .page-box {
    flex: 1;
    min-height: 0;
  }
  .page-link {
    padding: var(--space-3);
    overflow-wrap: anywhere;
  }
</style>
