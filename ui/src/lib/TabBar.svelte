<script lang="ts">
  import { popup } from './popup';
  // **The one bar** (`decisions.md` 2026-11-17, *one frame*): `≡` (formicaria's options), the tabs
  // (every open feature, note and page, one row), `+` (a new tab, suggesting the features). At the
  // bottom by default, at the top when the person chooses (Safari's and Firefox's choice). A tab
  // dragged into the screen shows with the one in front (tiling); ✕ on the tab in front closes it.
  import Icon from './Icon.svelte';
  import type { Pane } from './panes';
  import { search } from './ipc';
  import { notch, place, rolling, settle, SIDE } from './roller';

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
    onprivacy,
    onfire,
    onbackup,
    onundo,
    onsavepage,
    onsavemedia,
    roller = false,
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
    /** ≡ → Privacy: the switches, and what they did on the page in front. */
    onprivacy?: () => void;
    /** ≡ → 🔥 Forget web data (the phone, 2026-11-28): asks, then closes the web's tabs and forgets. */
    onfire?: () => void;
    onbackup: () => void;
    onundo: () => void;
    /** A web page is in front: *Save this page* (its article, in a note of its own). */
    onsavepage?: () => void;
    /** A web page is in front: *Save media* (its video or sound, into Music, Videos or a note). */
    onsavemedia?: () => void;
    /** **The roller** (`roller.ts`): the open items on an arc, rolled with the thumb (the phone). */
    roller?: boolean;
  } = $props();

  /** A roll in progress: where it began, how far it went, and along which axis it was read. */
  let roll = $state<{
    x0: number;
    y0: number;
    dx: number;
    dy: number;
    axis: 'x' | 'y' | null;
    id: string | null;
  } | null>(null);
  let rolled = false;
  const mid = $derived(roll?.axis === 'x' ? rolling(focused, roll.dx, panes.length) : focused);
  function rollStart(e: PointerEvent) {
    const li = (e.target as Element).closest('[data-id]') as HTMLElement | null;
    roll = { x0: e.clientX, y0: e.clientY, dx: 0, dy: 0, axis: null, id: li?.dataset.id ?? null };
    rolled = false;
  }
  function rollMove(e: PointerEvent) {
    if (!roll) return;
    const dx = e.clientX - roll.x0;
    const dy = e.clientY - roll.y0;
    // Read once it is clearly one way: sideways rolls, up (or down, from a top bar) pairs.
    const axis =
      roll.axis ?? (Math.hypot(dx, dy) > 8 ? (Math.abs(dx) >= Math.abs(dy) ? 'x' : 'y') : null);
    // Following the finger past the bar is a nicety: a browser that refuses it (a pointer it does not
    // know) must not stop the roll — in the app's web view that refusal threw, and nothing rolled.
    if (axis === 'x') {
      try {
        (e.currentTarget as Element).setPointerCapture?.(e.pointerId);
      } catch {
        /* rolled without it */
      }
    }
    roll = { ...roll, dx, dy, axis };
  }
  function rollEnd() {
    if (!roll) return;
    if (roll.axis === 'x') {
      rolled = true;
      const to = settle(focused, roll.dx, panes.length);
      if (to !== focused) onselect(to);
    } else if (roll.axis === 'y' && roll.id && into(roll.dy)) {
      rolled = true;
      const i = panes.findIndex((p) => p.id === roll!.id);
      if (i >= 0 && i !== focused) onpair(roll.id);
    }
    roll = null;
  }
  function rollWheel(e: WheelEvent) {
    e.preventDefault();
    const to = notch(focused, e.deltaY || e.deltaX, panes.length);
    if (to !== focused) onselect(to);
  }

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

  // The tab in front is kept in view as the row slides (+ on a feature already open lands there).
  let row = $state<HTMLUListElement>();
  $effect(() => {
    void focused;
    void panes.length;
    const on = row?.querySelector('.tab.on') as HTMLElement | null;
    on?.scrollIntoView?.({ inline: 'nearest', block: 'nearest' });
  });
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
        {#if onsavepage}
          <button type="button" role="menuitem" onclick={act(onsavepage)}>Save this page</button>
        {/if}
        {#if onsavemedia}
          <button type="button" role="menuitem" onclick={act(onsavemedia)}>Save media</button>
        {/if}
        <button type="button" role="menuitem" onclick={act(onundo)}>Undo</button>
        <button type="button" role="menuitem" onclick={act(onbackup)}>Back up and sync</button>
        {#if onprivacy}
          <button type="button" role="menuitem" onclick={act(onprivacy)}>Privacy</button>
        {/if}
        {#if onfire}
          <button type="button" role="menuitem" onclick={act(onfire)}>🔥 Forget web data</button>
        {/if}
        <button type="button" role="menuitem" onclick={act(onsettings)}>Settings</button>
        <button type="button" role="menuitem" onclick={act(onhelp)}>Help</button>
      </div>
    {/if}
  </div>
  {#if roller}
    <ul
      class="roller"
      class:live={roll?.axis === 'x'}
      aria-label="open tabs"
      onpointerdown={rollStart}
      onpointermove={rollMove}
      onpointerup={rollEnd}
      onpointercancel={() => (roll = null)}
      onwheel={rollWheel}
    >
      {#each panes as p, i (p.id)}
        {@const ic = icon(p)}
        {@const g = place(i - mid)}
        {#if Math.abs(i - mid) <= SIDE + 1}
          <li
            class="tab rolled"
            class:on={i === focused}
            data-id={p.id}
            style:transform="translateX(-50%) translate({g.x}px, {g.y}px) rotate({g.rotate}deg)
            scale({g.scale})"
            style:opacity={g.opacity}
            style:z-index={10 - Math.round(Math.abs(i - mid))}
          >
            <button
              type="button"
              class="pick"
              aria-current={i === focused ? 'page' : undefined}
              onclick={() => {
                if (!rolled) onselect(i);
                rolled = false;
              }}
            >
              {#if ic}<Icon name={ic} size={14} />{/if}
              <span>{title(p)}</span>
            </button>
            {#if i === focused}
              <button
                type="button"
                class="close"
                aria-label="close {title(p)}"
                onclick={() => onclose(p.id)}>×</button
              >
            {/if}
          </li>
        {/if}
      {/each}
    </ul>
  {:else}
    <ul class="tabs" aria-label="open tabs" bind:this={row}>
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
          {#if i === focused}
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
  {/if}
  <button type="button" class="plus" aria-label="new tab" onclick={onnew}>+</button>
</nav>

<style>
  /* The roller: the open items on an arc, the one in front in the middle (`roller.ts`). */
  .roller {
    position: relative;
    flex: 1;
    min-width: 0;
    height: 3.1rem;
    margin: 0;
    padding: 0;
    list-style: none;
    overflow: hidden;
    touch-action: none;
  }
  .roller .rolled {
    position: absolute;
    left: 50%;
    top: 0.2rem;
    max-width: 10rem;
    display: flex;
    align-items: center;
    transform-origin: 50% 160%;
    transition:
      transform 0.18s ease-out,
      opacity 0.18s ease-out;
  }
  .roller.live .rolled {
    transition: none;
  }
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
  /* **While anything is typed with the keyboard up, the bar steps aside** (`lib/barAside.ts` sets
     `data-typing`; `decisions.md` 2026-11-24). It sat between the text and the keys, taking a slice
     of what little the keyboard leaves. Its own search keeps it: that field is not counted. The
     keyboard put away brings it back.

     **It slides, it does not blink** (the owner: *"A seamless transition"*): the height goes to
     nothing and the page grows into it. `interpolate-size` lets `auto` be animated where the engine
     knows it; elsewhere the bar simply goes. Only the hidden state sets a height, so a bar that wraps
     is never clipped when shown. `visibility` waits for the slide before taking the controls out of
     reach, and comes back at once. The page takes over the inset the bar was padding
     (`App.svelte`, `.tab-shell`). */
  .tab-bar {
    interpolate-size: allow-keywords;
    transition:
      height 180ms ease,
      padding 180ms ease,
      border-width 180ms ease,
      visibility 0s linear 0s;
  }
  :global(html[data-typing]) .tab-bar {
    height: 0;
    padding-top: 0;
    padding-bottom: 0;
    border-width: 0;
    overflow: hidden;
    visibility: hidden;
    transition:
      height 180ms ease,
      padding 180ms ease,
      border-width 180ms ease,
      visibility 0s linear 180ms;
  }
  /* Under reduced motion it just goes and comes back: the global rule shortens durations, not the
     wait before the controls are out of reach. */
  @media (prefers-reduced-motion: reduce) {
    .tab-bar,
    :global(html[data-typing]) .tab-bar {
      transition: none;
    }
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
