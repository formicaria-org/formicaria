<script lang="ts">
  import { popup } from './popup';
  // **The one bar** (`decisions.md` 2026-11-17, *one frame*): `≡` (formicaria's options), the tabs
  // (every open feature, note and page, one row), `+` (a new tab, suggesting the features). At the
  // bottom by default, at the top when the person chooses (Safari's and Firefox's choice). A tab
  // dragged into the screen shows with the one in front (tiling); ✕ on the tab in front closes it.
  import Icon from './Icon.svelte';
  import type { Pane } from './panes';
  import { search } from './ipc';

  let {
    panes,
    focused,
    besideIndex,
    position,
    title,
    icon,
    onselect,
    onclose,
    onpair,
    onnew,
    onopennote,
    onsettings,
    onhelp,
    onbackup,
    onundo,
    onposition,
  }: {
    panes: Pane[];
    focused: number;
    besideIndex: number;
    position: 'top' | 'bottom';
    title: (p: Pane) => string;
    icon: (p: Pane) => string | null;
    onselect: (i: number) => void;
    onclose: (id: string) => void;
    onpair: (id: string) => void;
    onnew: () => void;
    onopennote: (id: string) => void;
    onsettings: () => void;
    onhelp: () => void;
    onbackup: () => void;
    onundo: () => void;
    onposition: (p: 'top' | 'bottom') => void;
  } = $props();

  let menuOpen = $state(false);
  let q = $state('');
  let hits = $state<{ id: string; title: string }[]>([]);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let seq = 0;
  /** Debounced, as the app's other search: typing a word is a query or two, never one per key. */
  function find() {
    clearTimeout(timer);
    const t = q.trim();
    if (!t) return (hits = []);
    timer = setTimeout(async () => {
      const mine = ++seq;
      try {
        const got = (await search(t))
          .slice(0, 8)
          .map((n) => ({ id: n.id, title: n.title || 'Untitled' }));
        if (mine === seq) hits = got;
      } catch {
        if (mine === seq) hits = [];
      }
    }, 250);
  }
  const act = (f: () => void) => () => {
    menuOpen = false;
    f();
  };

  /** A tab dragged into the screen (up from a bottom bar, down from a top one) tiles with the one in front. */
  let drag: { id: string; y0: number; dy: number } | null = $state(null);
  const into = (dy: number) => (position === 'bottom' ? dy < -60 : dy > 60);
</script>

<nav class="tab-bar" class:top={position === 'top'} aria-label="formicaria">
  <div class="menu-wrap">
    <button
      type="button"
      class="menu-btn"
      aria-label="formicaria's options"
      aria-haspopup="menu"
      aria-expanded={menuOpen}
      onclick={() => (menuOpen = !menuOpen)}>≡</button
    >
    {#if menuOpen}
      <div class="menu-backdrop" role="presentation" onclick={() => (menuOpen = false)}></div>
      <div class="menu" role="menu" aria-label="formicaria's options" use:popup>
        <input
          type="search"
          placeholder="Search your notes"
          aria-label="search your notes"
          bind:value={q}
          oninput={find}
        />
        {#each hits as h (h.id)}
          <button type="button" role="menuitem" onclick={act(() => onopennote(h.id))}
            >{h.title}</button
          >
        {/each}
        <button type="button" role="menuitem" onclick={act(onundo)}>Undo</button>
        <button type="button" role="menuitem" onclick={act(onbackup)}>Back up and sync</button>
        <button type="button" role="menuitem" onclick={act(onsettings)}>Settings</button>
        <button type="button" role="menuitem" onclick={act(onhelp)}>Help</button>
        <button
          type="button"
          role="menuitem"
          onclick={act(() => onposition(position === 'bottom' ? 'top' : 'bottom'))}
          >This bar at the {position === 'bottom' ? 'top' : 'bottom'}</button
        >
      </div>
    {/if}
  </div>
  <ul class="tabs" aria-label="open tabs">
    {#each panes as p, i (p.id)}
      {@const ic = icon(p)}
      <li
        class="tab"
        class:on={i === focused}
        class:beside={i === besideIndex}
        style:transform={drag?.id === p.id ? `translateY(${drag.dy}px)` : undefined}
        onpointerdown={(e) => (drag = { id: p.id, y0: e.clientY, dy: 0 })}
        onpointermove={(e) => {
          if (drag?.id === p.id) drag = { ...drag, dy: e.clientY - drag.y0 };
        }}
        onpointerup={() => {
          if (drag?.id === p.id && into(drag.dy) && i !== focused) onpair(p.id);
          drag = null;
        }}
        onpointercancel={() => (drag = null)}
      >
        <button
          type="button"
          class="pick"
          aria-current={i === focused ? 'page' : undefined}
          onclick={() => {
            if (!drag || Math.abs(drag.dy) < 10) onselect(i);
          }}
        >
          {#if ic}<Icon name={ic} size={14} />{/if}
          <span>{title(p)}</span>
        </button>
        {#if i === focused && panes.length > 1}
          <button
            type="button"
            class="close"
            aria-label="close {title(p)}"
            onclick={() => onclose(p.id)}>×</button
          >
        {/if}
      </li>
    {/each}
  </ul>
  <button type="button" class="plus" aria-label="new tab" onclick={onnew}>+</button>
</nav>

<style>
  .tab-bar {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-1) var(--space-1) max(var(--safe-bottom, 0px), var(--space-1));
    padding-inline: max(var(--safe-left, 0px), var(--space-1))
      max(var(--safe-right, 0px), var(--space-1));
    background: var(--bg-raised, var(--bg, Canvas));
    border-top: 1px solid color-mix(in srgb, currentColor 12%, transparent);
    min-width: 0;
  }
  .tab-bar.top {
    padding-top: max(var(--safe-top, 0px), var(--space-1));
    padding-bottom: var(--space-1);
    border-top: 0;
    border-bottom: 1px solid color-mix(in srgb, currentColor 12%, transparent);
  }
  .menu-btn,
  .plus {
    flex: none;
    min-width: 2.6rem;
    min-height: 2.6rem;
    font-size: 1.3rem;
  }
  .tabs {
    flex: 1;
    min-width: 0;
    display: flex;
    gap: 4px;
    overflow-x: auto;
    list-style: none;
    margin: 0;
    padding: 0;
    scrollbar-width: none;
  }
  .tab {
    flex: none;
    display: flex;
    align-items: center;
    max-width: 11rem;
    border-radius: 10px;
    touch-action: pan-x;
  }
  .tab.on {
    background: color-mix(in srgb, #dc2626 16%, transparent);
  }
  .tab.beside {
    outline: 1px dashed color-mix(in srgb, #dc2626 60%, transparent);
  }
  .pick {
    display: flex;
    align-items: center;
    gap: 4px;
    min-height: 2.4rem;
    padding: 0 var(--space-2);
    min-width: 0;
  }
  .pick span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .close {
    min-width: 2rem;
    min-height: 2.4rem;
  }
  .menu-wrap {
    position: relative;
    flex: none;
  }
  .menu-backdrop {
    position: fixed;
    inset: 0;
    z-index: 40;
  }
  .menu {
    width: min(18rem, 90vw);
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px;
    border-radius: 12px;
    background: var(--bg, Canvas);
    box-shadow: 0 6px 24px rgba(0, 0, 0, 0.28);
  }
  .menu input {
    padding: 7px 9px;
    margin-bottom: 4px;
  }
  .menu button {
    text-align: start;
    padding: 9px 10px;
    border-radius: 6px;
  }
</style>
