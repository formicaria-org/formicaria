<script lang="ts">
  // One pane of the workspace: a compact header (view picker, per-view controls, close) over
  // one of the existing renderers. The renderers are pure and already `height:100%`, so a pane
  // is a thin frame around them — the whole reason the flexible grid is cheap.
  import {
    draggable,
    dropTargetForElements,
  } from '@atlaskit/pragmatic-drag-and-drop/element/adapter';
  import Board from '../renderers/Board.svelte';
  import Agenda from '../renderers/Agenda.svelte';
  import Calendar from '../renderers/Calendar.svelte';
  import Timeline from '../renderers/Timeline.svelte';
  import Search from '../renderers/Search.svelte';
  import Activity from '../renderers/Activity.svelte';
  import Icon from './Icon.svelte';
  import type { Pane, PaneKind, Feed } from './panes';
  import { paneTitle, clampSpan } from './panes';
  import type { ObjectMeta, ViewInfo, Board as BoardT } from './types';
  import { orderColumns, moveValue } from './boardOrder';

  interface Props {
    pane: Pane;
    feed: Feed | undefined; // the fetched data for this pane's feed (undefined = loading)
    index: number; // this pane's slot in the workspace (drag source/target identity)
    cols: number; // the workspace column count (clamps a resize)
    statuses: string[];
    savedViews: ViewInfo[];
    shown: (n: { id: string; vault: string }) => boolean;
    focused: boolean;
    startEditing: boolean; // a note pane opened via "New note" starts in the editor
    vaults: string[]; // all vault names — a note pane offers "Copy to" the others
    onopen: (id: string) => void;
    onmove: (groupBy: string, id: string, value: string, beforeId: string | null) => void;
    onstatus: (id: string, value: string | null) => void;
    onnavigate: (id: string) => void; // a note pane's chip was followed → open that note
    onsaved: () => void; // a note pane wrote → schedule the git commit
    onchange: (patch: Partial<Pane>) => void;
    onreorder: (from: number, to: number) => void;
    onresize: (patch: { colSpan?: number; rowSpan?: number }) => void;
    onclose: () => void;
    onfocus: () => void;
  }
  let {
    pane,
    feed,
    index,
    cols,
    statuses,
    savedViews,
    shown,
    focused,
    startEditing,
    vaults,
    onopen,
    onmove,
    onstatus,
    onnavigate,
    onsaved,
    onchange,
    onreorder,
    onresize,
    onclose,
    onfocus,
  }: Props = $props();

  // A note pane can "maximize" to fill the workspace (the old full-screen toggle, repurposed
  // as a span change). NotePanel's ⤢/⤡ button drives it; `wide` picks the icon.
  const maximized = $derived(pane.colSpan >= cols);
  function toggleMax() {
    onresize(maximized ? { colSpan: 1, rowSpan: 1 } : { colSpan: cols, rowSpan: 2 });
  }

  let dragover = $state(false);

  // The header grip is the drag handle; the whole pane is the drop target. A pane drag carries
  // its `paneIndex`; card/column drags (from a Board inside a pane) carry `id`/`columnValue`, so
  // the branches never cross — a card dropped on a pane is ignored here, a pane dropped on a
  // board column is ignored there. Same svelte-action shape as Board's column dnd.
  function gripDrag(node: HTMLElement, i: number) {
    let current = i;
    const cleanup = draggable({ element: node, getInitialData: () => ({ paneIndex: current }) });
    return { update: (v: number) => (current = v), destroy: cleanup };
  }
  function paneDrop(node: HTMLElement, i: number) {
    let current = i;
    const cleanup = dropTargetForElements({
      element: node,
      canDrop: ({ source }) => typeof source.data.paneIndex === 'number',
      onDragEnter: () => (dragover = true),
      onDragLeave: () => (dragover = false),
      onDrop: ({ source }) => {
        dragover = false;
        const from = source.data.paneIndex;
        if (typeof from === 'number' && from !== current) onreorder(from, current);
      },
    });
    return { update: (v: number) => (current = v), destroy: cleanup };
  }

  // Corner-drag resize: measure a single track from the pane's own rendered box (width /
  // current colSpan), then set the span to whatever the pointer now covers. Self-measured —
  // no access to the grid container needed — and clamped to the column count. Pointer capture
  // keeps the drag alive even as the pointer leaves the small grip.
  let paneEl = $state<HTMLElement | undefined>(undefined);
  function startResize(e: PointerEvent) {
    if (!paneEl) return;
    e.preventDefault();
    e.stopPropagation();
    const grip = e.currentTarget as HTMLElement;
    grip.setPointerCapture(e.pointerId);
    const rect = paneEl.getBoundingClientRect();
    const colW = rect.width / Math.max(1, pane.colSpan);
    const rowH = rect.height / Math.max(1, pane.rowSpan);
    const move = (ev: PointerEvent) => {
      const colSpan = clampSpan(Math.round((ev.clientX - rect.left) / colW), cols);
      const rowSpan = Math.max(1, Math.min(Math.round((ev.clientY - rect.top) / rowH), 4));
      if (colSpan !== pane.colSpan || rowSpan !== pane.rowSpan) onresize({ colSpan, rowSpan });
    };
    const up = (ev: PointerEvent) => {
      grip.releasePointerCapture(ev.pointerId);
      grip.removeEventListener('pointermove', move);
      grip.removeEventListener('pointerup', up);
    };
    grip.addEventListener('pointermove', move);
    grip.addEventListener('pointerup', up);
  }

  // Cards this pane shows, after the global vault filter.
  const cards = $derived((feed?.cards ?? []).filter(shown));
  // A board with its cards vault-filtered (columns kept, even if emptied).
  const board = $derived(
    feed?.board
      ? { ...feed.board, columns: feed.board.columns.map((c) => ({ ...c, cards: c.cards.filter(shown) })) }
      : null,
  );

  // The picker's options: the built-in kinds plus each saved `.view` (by name).
  const KINDS: { value: string; label: string }[] = [
    { value: 'board', label: 'Board' },
    { value: 'agenda', label: 'Agenda' },
    { value: 'timeline', label: 'Timeline' },
    { value: 'search', label: 'Search' },
    { value: 'activity', label: 'Activity' },
  ];

  function pick(value: string) {
    if (value.startsWith('view:')) {
      onchange({ kind: 'view', viewName: value.slice(5) });
    } else {
      onchange({ kind: value as PaneKind, viewName: null });
    }
  }
  const pickValue = $derived(pane.kind === 'view' ? `view:${pane.viewName}` : pane.kind);

  // The view picker is a *rotator*, not a dropdown: the same options in a ring the user steps
  // through by clicking (forward) or scrolling the wheel over it (either direction). Same
  // `pick()` write-back as the old <select>, just a different way to reach a value.
  const options = $derived([
    ...KINDS,
    ...savedViews.filter((v) => !v.error).map((v) => ({ value: `view:${v.name}`, label: v.name })),
  ]);
  const currentIndex = $derived(Math.max(0, options.findIndex((o) => o.value === pickValue)));
  const currentLabel = $derived(options[currentIndex]?.label ?? 'Board');
  function rotate(dir: number) {
    if (!options.length) return;
    pick(options[(currentIndex + dir + options.length) % options.length].value);
  }

  /// **The whole header rotates the view, not just the little label.**
  ///
  /// The rotator button worked and nobody found it — it is one small target among the header's
  /// controls, and nothing about it says "scroll me". The header is the biggest thing in a pane
  /// that is not content, it is already the drag handle, and it is where you look when you are
  /// thinking about *this window* rather than what is in it. So the wheel anywhere across it
  /// spins the view, and a horizontal swipe does the same on a touch screen.
  ///
  /// **Not the pane body.** A board scrolls horizontally by design and the agenda scrolls
  /// vertically; stealing those gestures from the content would trade one discoverable action
  /// for two broken ones.
  ///
  /// A note pane has no view to rotate, so it is exempt — `options` describes ways of looking at
  /// a *collection*, and a note is one document.
  const rotatable = $derived(pane.kind !== 'note');

  /// **One gesture is one step**, and the rule depends only on *how far* you scrolled.
  ///
  /// Two bugs came out of this control, both reported within minutes, and both were the same
  /// mistake in different clothes: the amount a step cost depended on **history** rather than on
  /// the gesture in front of it.
  ///
  /// 1. Rotating on every event — a wheel sends a burst per detent and a trackpad sends a stream
  ///    plus inertia, so a light movement span several views.
  /// 2. Then the cooldown, when it blocked a step, was *cleared by a reversal* but not by
  ///    continuing. So after any step the direction you were already going was throttled and the
  ///    opposite fired instantly: "rolling up seems to be sensitive differently than rolling
  ///    down". It was, by construction.
  ///
  /// So there is no direction memory here at all any more, and nothing carries between gestures.
  /// A wheel event is judged on its own size:
  ///
  /// - **A detent** — one click of a real wheel — turns the view once, at once, either way. You
  ///   cannot produce these faster than your fingers move, so throttling them only ever makes a
  ///   deliberate act feel ignored.
  /// - **Smooth scrolling** — a trackpad, or a high-resolution wheel — accumulates, because its
  ///   events are far smaller and far more numerous. Only there is a cooldown needed, and it
  ///   applies the same in both directions.

  /// Above this, a single event is a discrete detent. Chrome reports 100–120 for one, Firefox
  /// three lines (120 normalised); a trackpad's individual deltas are an order of magnitude less.
  const WHEEL_NOTCH = 50;
  /// How much smooth scrolling makes one step. Deliberately several times a detent: these arrive
  /// in long streams, and the original complaint was a barely-there movement spending views.
  const WHEEL_SMOOTH_STEP = 180;
  /// Only for smooth scrolling, and applied identically whichever way you are going — it exists
  /// to stop inertia banking steps after your fingers have left the surface, nothing else.
  const WHEEL_COOLDOWN = 220;
  /// Quiet long enough to end a gesture, so nothing you scrolled a moment ago is still owed.
  const WHEEL_GAP = 350;

  let wheelAccum = 0;
  let lastWheelAt = 0;
  let lastRotateAt = 0;

  /// A wheel delta in pixels, whichever unit the browser chose to report.
  ///
  /// `deltaMode` is lines on Firefox and pages in some configurations, so the raw number means
  /// nothing on its own. A line counts as 40px rather than the ~16 it really is, because Firefox
  /// sends **three** lines per detent — the number that matters is the one that makes a detent
  /// weigh the same in both browsers.
  function wheelPixels(e: WheelEvent): number {
    const scale = e.deltaMode === 1 ? 40 : e.deltaMode === 2 ? 800 : 1;
    return e.deltaY * scale;
  }

  function headWheel(e: WheelEvent) {
    if (!rotatable) return;
    // Only when the wheel is actually being turned vertically: a trackpad's horizontal flick is
    // how you scroll a board, and it reaches here when the pointer is over the header.
    if (Math.abs(e.deltaY) <= Math.abs(e.deltaX)) return;
    e.preventDefault();

    const now = performance.now();
    const dy = wheelPixels(e);
    const dir = dy > 0 ? 1 : -1;

    // A detent: unambiguous, deliberate, and answered immediately in either direction.
    if (Math.abs(dy) >= WHEEL_NOTCH) {
      wheelAccum = 0;
      lastWheelAt = now;
      lastRotateAt = now;
      rotate(dir);
      return;
    }

    // Smooth scrolling: add up, and start over after a pause so nothing is carried into the
    // next gesture.
    if (now - lastWheelAt > WHEEL_GAP) wheelAccum = 0;
    lastWheelAt = now;
    wheelAccum += dy;
    if (Math.abs(wheelAccum) < WHEEL_SMOOTH_STEP) return;
    // Discarded rather than held: holding it at the threshold is what pre-charged one direction
    // last time. Either way, a step always costs a whole gesture.
    wheelAccum = 0;
    if (now - lastRotateAt < WHEEL_COOLDOWN) return;
    lastRotateAt = now;
    rotate(dir);
  }


  // Swipe: recorded on the header only, and deliberately generous about what counts as one.
  let touchX = 0;
  let touchY = 0;
  /** Below this a swipe is a tap that wandered; ~1/8 of a phone's width. */
  const SWIPE_MIN = 48;
  function headTouchStart(e: TouchEvent) {
    const t = e.changedTouches[0];
    touchX = t?.clientX ?? 0;
    touchY = t?.clientY ?? 0;
  }
  function headTouchEnd(e: TouchEvent) {
    if (!rotatable) return;
    const t = e.changedTouches[0];
    if (!t) return;
    const dx = t.clientX - touchX;
    const dy = t.clientY - touchY;
    // Horizontal intent, not a vertical scroll that started on the header.
    if (Math.abs(dx) < SWIPE_MIN || Math.abs(dx) <= Math.abs(dy)) return;
    // Swipe left = forward, matching every carousel: the content moves the way your finger did.
    rotate(dx < 0 ? 1 : -1);
  }

  // **Column order, reconnected.** `boardOrder.ts` is a pure, tested core — and after the
  // pane rewrite nothing imported it: `onreorder` was `() => {}`, so dragging a column
  // header did nothing while the drag still started and the cursor still said `grab`. An
  // affordance that looks live and is dead is worse than no affordance.
  //
  // Keyed by **this pane's `groupBy`**, because the order of "todo/doing/done" has nothing
  // to say about the order of "project A/project B". Two panes grouped the same way share
  // one order, which is right — it is the same board seen twice.
  //
  // A view preference, so `localStorage` and never a vault: what you are currently looking
  // at is not knowledge.
  const ORDER_KEY = 'fm-board-order';

  function allOrders(): Record<string, string[]> {
    try {
      return JSON.parse(localStorage.getItem(ORDER_KEY) ?? '{}');
    } catch {
      return {};
    }
  }

  // Bumped on write so the `$derived` below re-runs — localStorage is not reactive.
  let orderVersion = $state(0);

  function ordered(b: BoardT): BoardT {
    void orderVersion;
    const saved = allOrders()[pane.groupBy];
    return saved?.length ? { ...b, columns: orderColumns(b.columns, saved) } : b;
  }

  function reorderColumns(fromValue: string, toValue: string, before: boolean) {
    if (!board) return;
    // Move within the order the user is *looking at*, not the server's — otherwise the
    // second drag is computed against a list that is no longer on screen. `moveValue`
    // returns the whole permutation, so the first drag persists a complete, stable order.
    const visible = ordered(board).columns.map((c) => c.value);
    const next = moveValue(visible, fromValue, toValue, before);
    try {
      localStorage.setItem(ORDER_KEY, JSON.stringify({ ...allOrders(), [pane.groupBy]: next }));
      orderVersion++;
    } catch {
      /* private mode — the order just won't persist */
    }
  }
</script>

<section
  class="pane"
  class:focused
  class:dragover
  bind:this={paneEl}
  use:paneDrop={index}
  onpointerdowncapture={onfocus}
  aria-label={paneTitle(pane)}
>
  <!-- The whole header is the drag handle (not just the grip) — grab anywhere on the top bar to
       move the pane. Clicks on the controls inside still register (a click is not a drag). -->
  <header
    class="pane-head"
    class:grip-only={pane.kind === 'note'}
    use:gripDrag={index}
    onwheel={headWheel}
    ontouchstart={headTouchStart}
    ontouchend={headTouchEnd}
    role="toolbar"
    tabindex="-1"
    aria-label="pane controls — drag to rearrange, scroll or swipe to change the view"
    title={rotatable
      ? 'Drag to rearrange · scroll or swipe here to change the view'
      : 'Drag to rearrange this pane'}
  >
    <span class="grip" aria-hidden="true">
      <Icon name="grip" size={13} />
    </span>
    <!-- A note pane wears NotePanel's own chrome (title, status, edit, delete, close), so the
         pane header shrinks to just the drag grip. Every other kind gets the view picker. -->
    {#if pane.kind !== 'note'}
      <!-- A rotator, not a dropdown: click to advance, scroll the wheel to spin either way. -->
      <button
        type="button"
        class="kind rotator"
        onclick={() => rotate(1)}
        title="Click, or scroll/swipe anywhere on this bar, to change the view"
        aria-label="pane view"
      >
        {currentLabel}
      </button>

      <!-- Per-view controls, inline in the pane header (each pane tunes itself). -->
      {#if pane.kind === 'board'}
        <input
          class="ctl group"
          list="pane-props"
          value={pane.groupBy}
          oninput={(e) => onchange({ groupBy: (e.currentTarget as HTMLInputElement).value })}
          spellcheck="false"
          title="group by"
          aria-label="group by"
        />
      {:else if pane.kind === 'agenda'}
        <div class="seg">
          <button class:on={pane.agendaMode === 'month'} onclick={() => onchange({ agendaMode: 'month' })}>M</button>
          <button class:on={pane.agendaMode === 'week'} onclick={() => onchange({ agendaMode: 'week' })}>W</button>
          <button class:on={pane.agendaMode === 'list'} onclick={() => onchange({ agendaMode: 'list' })}>L</button>
        </div>
      {:else if pane.kind === 'search'}
        <input
          class="ctl"
          type="search"
          placeholder="Search…"
          value={pane.query}
          oninput={(e) => onchange({ query: (e.currentTarget as HTMLInputElement).value })}
          spellcheck="false"
          aria-label="search"
        />
      {/if}
    {/if}

    <span class="spacer"></span>
    {#if pane.kind !== 'note'}
      <button class="pane-close" onclick={onclose} aria-label="close pane" title="Close pane">
        <Icon name="close" size={14} />
      </button>
    {/if}
  </header>

  <div class="pane-body" class:note-body={pane.kind === 'note'}>
    {#if pane.kind === 'note'}
      {#if pane.noteId}
        {#await import('./NotePanel.svelte') then { default: NotePanel }}
          <NotePanel
            id={pane.noteId}
            {statuses}
            {startEditing}
            {vaults}
            solo
            wide={maximized}
            onclose={onclose}
            onnavigate={onnavigate}
            onsaved={onsaved}
            ontogglewide={toggleMax}
          />
        {/await}
      {:else}
        <p class="pane-empty">No note.</p>
      {/if}
    {:else if pane.kind === 'board' || (pane.kind === 'view' && board)}
      {#if board}
        <Board
          board={ordered(board)}
          onmove={(id, value, beforeId) => onmove(pane.groupBy, id, value, beforeId)}
          onreorder={reorderColumns}
          {onopen}
          {statuses}
          onstatus={onstatus}
        />
      {:else}
        <p class="pane-empty">Loading…</p>
      {/if}
    {:else if pane.kind === 'search'}
      <Search {cards} query={pane.query} {onopen} />
    {:else if pane.kind === 'activity'}
      <Activity {shown} {onopen} />
    {:else if pane.kind === 'agenda'}
      {#if pane.agendaMode === 'list'}
        <Agenda {cards} {onopen} />
      {:else}
        <Calendar {cards} {onopen} range={pane.agendaMode === 'week' ? 'week' : 'month'} />
      {/if}
    {:else}
      <!-- timeline, or a flat-list saved view -->
      <Timeline {cards} {onopen} {statuses} onstatus={onstatus} />
    {/if}
  </div>

  <!-- Corner resize handle: drag to change how many columns/rows this pane spans. -->
  <span
    class="resize-grip"
    role="slider"
    tabindex="-1"
    aria-label="resize pane"
    aria-valuenow={pane.colSpan}
    title="Drag to resize"
    onpointerdown={startResize}
  ></span>
</section>

<datalist id="pane-props">
  <option value="status"></option>
  <option value="project"></option>
  <option value="tags"></option>
</datalist>

<style>
  .pane {
    /* Renderers inside a pane must size to *the pane*, not the window. A colSpan:1 pane in a
       4-column workspace on a wide monitor is ~380px, and viewport media queries were handing
       it desktop-width board columns it could not fit; a maximised pane on a small window got
       `85vw` columns inside a box that was not 85vw. The media queries were asking the wrong
       element. This is what lets one renderer be correct at any size — and it is the capability
       that did not exist when the standard advice was "build a separate mobile site". */
    container-type: inline-size;
    container-name: pane;

    position: relative;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    overflow: hidden;
  }
  .pane.focused {
    border-color: var(--accent);
  }
  /* Drop affordance while another pane is dragged over this one. */
  .pane.dragover {
    border-color: var(--accent);
    box-shadow: inset 0 0 0 2px var(--accent);
  }
  .grip {
    display: inline-flex;
    align-items: center;
    flex: none;
    color: var(--text-muted);
    cursor: grab;
    touch-action: none;
  }
  .grip:active {
    cursor: grabbing;
  }
  /* A small triangular handle in the bottom-right corner; drag it to resize. */
  .resize-grip {
    position: absolute;
    right: 0;
    bottom: 0;
    width: 16px;
    height: 16px;
    cursor: nwse-resize;
    touch-action: none;
    z-index: 2;
    background: linear-gradient(
      135deg,
      transparent 0 50%,
      var(--border) 50% 62%,
      transparent 62% 74%,
      var(--border) 74% 86%,
      transparent 86%
    );
  }
  .resize-grip:hover {
    background: linear-gradient(
      135deg,
      transparent 0 50%,
      var(--accent) 50% 62%,
      transparent 62% 74%,
      var(--accent) 74% 86%,
      transparent 86%
    );
  }
  .pane-head {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 4px 6px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
    flex: none;
    cursor: grab; /* the whole header is the drag handle */
    user-select: none;
    touch-action: none;
  }
  .pane-head:active {
    cursor: grabbing;
  }
  /* The interactive controls sit above the header's grab cursor — they keep their own. */
  .pane-head .rotator,
  .pane-head .seg button,
  .pane-head .pane-close {
    cursor: pointer;
  }
  .pane-head .ctl {
    cursor: auto;
  }
  .kind {
    font: inherit;
    font-size: 0.85rem;
    background: var(--surface);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 2px 4px;
  }
  .rotator {
    min-width: 5rem;
    text-align: left;
    white-space: nowrap;
  }
  .rotator:hover {
    border-color: var(--accent);
  }
  .ctl {
    font: inherit;
    font-size: 0.85rem;
    min-width: 0;
    width: 8rem;
    background: var(--surface);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 2px 6px;
  }
  .seg {
    display: inline-flex;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }
  .seg button {
    font: inherit;
    font-size: 0.8rem;
    padding: 2px 7px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
  }
  .seg button.on {
    background: var(--accent);
    color: var(--accent-contrast);
  }
  .spacer {
    flex: 1;
  }
  .pane-close {
    flex: none;
    display: inline-flex;
    align-items: center;
    padding: 2px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    border-radius: var(--radius-sm);
  }
  .pane-close:hover {
    color: var(--text);
    background: var(--surface-hover);
  }
  .pane-body {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  /* A note pane hands its whole body to NotePanel, which scrolls itself (height:100%,
     width:100%). No pane-level scroll, no padding — the note owns the space. */
  .pane-body.note-body {
    display: flex;
    overflow: hidden;
    padding: 0;
  }
  .pane-body.note-body > :global(.panel) {
    box-shadow: none;
  }
  .pane-empty {
    padding: var(--space-4);
    color: var(--text-muted);
  }
</style>
