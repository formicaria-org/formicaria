<script lang="ts">
  // **A web page, an item of the collection like a note** (`docs/context/workspace-design-2026-11-15.md`
  // §13.7; `decisions.md` 2026-11-16, *a page is a layer*). On the phone the Android shell draws the
  // page itself, exactly over this box (`__fmPage`, `ci/android-inject-service.sh`); the app only says
  // where, and hides it whenever this box is out of sight (another item in front, a sheet over it).
  // Elsewhere (a computer, where formicaria's browser has real tabs) it is the address, to open.
  import { onDestroy } from 'svelte';
  import { webTarget } from './features';
  import { searchUrl, searchName } from './privacy.svelte';
  import type { Pane } from './panes';

  interface PageShell {
    show(id: string, url: string, x: number, y: number, w: number, h: number): void;
    hide(id: string): void;
    back(id: string): void;
    bring(id: string, note: string): void;
    go?(id: string, url: string): void;
    reload?(id: string): void;
  }
  const pageShell = (): PageShell | null =>
    (globalThis as { __fmPage?: PageShell }).__fmPage ?? null;

  let {
    pane,
  }: {
    pane: Pane;
    /** No longer used: what is brought in from a page goes by Android's selection bar. */
    target?: () => { id: string; title: string } | null;
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

  // **The address row** (the owner on 0.50: *"if I open a webpage I am stuck there, I can't continue
  // browsing since the search goes away"*): the page's address, which is also where to type a new
  // one or a search, and reload. Above the page, never over it; Android's Back goes back.
  let typing = $state(false);
  let typed = $state('');
  let field = $state<HTMLInputElement>();
  function edit() {
    typed = pane.url ?? '';
    typing = true;
    queueMicrotask(() => field?.select());
  }
  function goTo() {
    const t = webTarget(typed, searchUrl);
    typing = false;
    if (t && shell?.go) shell.go(pane.id, t.url);
  }

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
    <!-- **Nothing over the page** (the owner on 0.48: the ⇥ "is weird and should be removed", the ‹
         "why should I need it?"): Android's Back goes back in the page; what is selected is brought in
         from Android's selection bar (*Bring into note*, first), an image or a link by a long press;
         the tab in the bar names the page. -->
    <form
      class="address"
      onsubmit={(e) => {
        e.preventDefault();
        goTo();
      }}
    >
      {#if typing}
        <input
          bind:this={field}
          bind:value={typed}
          type="search"
          enterkeyhint="go"
          aria-label="a web address or a search"
          placeholder={`An address, or words to search with ${searchName()}`}
          onblur={() => (typing = false)}
        />
      {:else}
        <button type="button" class="where" aria-label="this page's address" onclick={edit}
          >{pane.url?.startsWith('fmsaved:')
            ? `Saved: ${pane.pageTitle ?? ''}`
            : host || 'a web page'}</button
        >
      {/if}
      <button
        type="button"
        class="reload"
        aria-label="reload the page"
        onclick={() => shell?.reload?.(pane.id)}>↻</button
      >
    </form>
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
  .address {
    display: flex;
    gap: 4px;
    align-items: center;
    padding: 3px var(--space-2);
    flex: none;
  }
  .address input,
  .where {
    flex: 1;
    min-width: 0;
    min-height: 2.2rem;
    border-radius: 999px;
    padding: 0 var(--space-3);
    border: 1px solid var(--border, #8884);
    background: var(--panel, #8881);
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .reload {
    min-width: 2.4rem;
    min-height: 2.2rem;
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
