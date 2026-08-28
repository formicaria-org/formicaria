<script lang="ts">
  import {
    draggable,
    dropTargetForElements,
  } from '@atlaskit/pragmatic-drag-and-drop/element/adapter';
  import Card from './Card.svelte';
  import type { Board } from '../lib/types';

  let { board, onmove, onreorder, onopen, statuses = [], onstatus }: {
    board: Board;
    /**
     * Drop a card into a column: set the grouped property to `value`, and place
     * the card immediately before `beforeId` (null = at the end of the column).
     */
    onmove: (id: string, value: string, beforeId: string | null) => void;
    /** Reposition a column (`fromValue`) before/after another (`toValue`). */
    onreorder: (fromValue: string, toValue: string, before: boolean) => void;
    onopen: (id: string) => void;
    statuses?: string[];
    onstatus?: (id: string, value: string | null) => void;
  } = $props();

  let over = $state<string | null>(null);

  /// The scrolling strip, and where along it we are — what the rail below the board reads.
  ///
  /// Measured from the DOM rather than tracked as state, the same discipline as `dropBefore` and
  /// the column-reorder edge: the browser owns the scroll position, and a second copy of it in a
  /// variable is a copy that goes stale on a resize, a font change or a snap the finger did.
  let strip = $state<HTMLElement | null>(null);
  let at = $state(0);
  let overflowing = $state(false);

  /// Which column is at the left edge, and is there anything past the edge at all. Runs on scroll,
  /// on a resize, and whenever the columns change — every way the answer can move.
  function measure() {
    if (!strip) return;
    // `+1` absorbs sub-pixel widths: a strip that fits exactly reports a scrollWidth a fraction
    // larger on fractional-DPI screens, and a rail that appears when nothing is hidden is noise.
    overflowing = strip.scrollWidth > strip.clientWidth + 1;
    const cols = strip.querySelectorAll<HTMLElement>('.column');
    if (!cols.length) return;
    // Rects, not `offsetLeft`: offsets are relative to whichever ancestor happens to be
    // positioned, and this component does not control that. Distances are comparable either way.
    const edge = strip.getBoundingClientRect().left;
    let best = 0;
    let closest = Infinity;
    cols.forEach((el, i) => {
      const d = Math.abs(el.getBoundingClientRect().left - edge);
      if (d < closest) {
        closest = d;
        best = i;
      }
    });
    at = best;
  }

  /// Bring column `i` to the left edge. `scrollIntoView` is guarded because jsdom does not
  /// implement it — the test asserts the call, and the real scrolling is a thing for the eye.
  function jump(i: number) {
    const el = strip?.querySelectorAll<HTMLElement>('.column')[i];
    el?.scrollIntoView?.({ inline: 'start', block: 'nearest', behavior: 'smooth' });
    at = i;
  }

  // Re-measure when the column set changes (a filter, a vault toggle, a card moved) and once on
  // mount. Reading `.length` is what subscribes this effect to it.
  $effect(() => {
    board.columns.length;
    measure();
  });

  // And when the *pane* resizes, which a window listener would miss: panes are resized by dragging
  // one corner, and the column width itself changes at a container breakpoint. Guarded because
  // jsdom ships no `ResizeObserver` — there, the effect above is the only trigger, which is all a
  // layout-free environment could honour anyway.
  $effect(() => {
    if (!strip || typeof ResizeObserver === 'undefined') return;
    const ro = new ResizeObserver(() => measure());
    ro.observe(strip);
    return () => ro.disconnect();
  });

  /**
   * Which card does a drop at `clientY` land above? The first card whose vertical
   * midpoint is below the pointer — i.e. the one that gets pushed down. Null when
   * the pointer is past every card, meaning "append". Self-measured from the DOM,
   * the same way the column-reorder branch below computes its edge, so there is
   * no hitbox registry to keep in sync. The dragged card is skipped: it is still
   * in the DOM at its old place, and measuring it would let a card block itself.
   */
  function dropBefore(node: HTMLElement, clientY: number, dragged: string): string | null {
    const cards = node.querySelectorAll<HTMLElement>('[data-card-id]');
    for (const el of cards) {
      const id = el.dataset.cardId;
      if (!id || id === dragged) continue;
      const rect = el.getBoundingClientRect();
      if (clientY < rect.top + rect.height / 2) return id;
    }
    return null;
  }

  // Each column is a drop target. Its identity is the grouped property's value —
  // an opaque string this renderer never inspects. A drop carries either a card
  // id (move the card → parent calls set_property(id, groupBy, value)) or a
  // columnValue (reorder the columns). No particular property/status name here.
  function column(node: HTMLElement, value: string) {
    let current = value;
    const cleanup = dropTargetForElements({
      element: node,
      getData: () => ({ value: current }),
      onDragEnter: () => (over = current),
      onDragLeave: () => {
        if (over === current) over = null;
      },
      onDrop: ({ source, location }) => {
        over = null;
        const id = source.data.id;
        if (typeof id === 'string') {
          onmove(id, current, dropBefore(node, location.current.input.clientY, id));
          return;
        }
        // Column reorder: insert before/after this column based on which half of
        // it the pointer is over (self-computed edge — no hitbox dependency).
        const from = source.data.columnValue;
        if (typeof from === 'string' && from !== current) {
          const rect = node.getBoundingClientRect();
          const before = location.current.input.clientX < rect.left + rect.width / 2;
          onreorder(from, current, before);
        }
      },
    });
    return {
      update: (v: string) => (current = v),
      destroy: cleanup,
    };
  }

  // The column header is the drag handle for reordering the whole column.
  function columnDrag(node: HTMLElement, value: string) {
    let current = value;
    const cleanup = draggable({
      element: node,
      getInitialData: () => ({ columnValue: current }),
    });
    return {
      update: (v: string) => (current = v),
      destroy: cleanup,
    };
  }
</script>

<!-- **A board that scrolls sideways has to admit it.** In a narrow pane a column fills the width
     and the strip snaps one at a time, so a fourth column is three swipes away with nothing on
     screen to say it exists — which reads exactly like a column that is not there (reported
     2026-08-24). The rail is the map: every column by name, the one you are on marked, and a tap
     jumps to it. Generic, like the rest of this renderer — the names are `col.label`, data the
     board never inspects. -->
<div class="board-wrap" class:overflowing>
  <div class="board" bind:this={strip} onscroll={measure}>
  {#each board.columns as col (col.value)}
    <section class="column" class:over={over === col.value} use:column={col.value}>
      <header
        class="column-head"
        data-value={col.value}
        use:columnDrag={col.value}
        title="Drag to reorder"
      >
        <span class="column-label">{col.label}</span>
        <span class="count">{col.cards.length}</span>
      </header>
      <div class="column-body">
        {#each col.cards as card (card.id)}
          <!-- `columns`/`onmoveto` are the touch stand-in for dragging: the element adapter
               above is HTML5 drag, which never fires on touch, and the package ships no
               pointer adapter to swap in. The menu reuses this very `onmove` — appending
               (`beforeId: null`), since a tap expresses a column, not a position — so a
               phone move and a desktop drop are the same write. -->
          <Card
            {card}
            {onopen}
            {statuses}
            {onstatus}
            columns={board.columns.map((c) => ({ value: c.value, label: c.label }))}
            column={col.value}
            onmoveto={(id, value) => onmove(id, value, null)}
          />
        {/each}
      </div>
    </section>
  {/each}
  </div>

  {#if board.columns.length > 1}
    <!-- Always in the DOM, shown by CSS: whether it is *needed* is a question about layout, and
         layout is the one thing the tests cannot see (jsdom applies no CSS and measures nothing).
         Rendering it unconditionally keeps the rail itself testable — its entries, their names and
         where a tap goes — and leaves only "is it visible" to the eye. -->
    <nav class="rail" aria-label="columns on this board">
      {#each board.columns as col, i (col.value)}
        <button
          type="button"
          class="rail-stop"
          class:on={i === at}
          aria-current={i === at ? 'true' : undefined}
          onclick={() => jump(i)}
        >
          <!-- The separator is deliberate: it keeps a rail entry's own text ("Backlog ·") from
               reading as the identical string to its column header ("Backlog"). The rail names
               every column, so without it every column label would appear twice in the document —
               ambiguous for anything, or anyone, searching the page by name. -->
          {col.label} ·<span class="rail-count">{col.cards.length}</span>
        </button>
      {/each}
    </nav>
  {/if}
</div>

<style>
  /* The wrap owns the pane's height; the strip takes what is left after the rail. `min-height: 0`
     on both, or the flex child refuses to shrink and the rail is pushed out of the pane. */
  .board-wrap {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    box-sizing: border-box;
  }
  .board {
    display: flex;
    gap: 0.85rem;
    align-items: flex-start;
    overflow-x: auto;
    padding: 1rem;
    flex: 1 1 auto;
    min-height: 0;
    box-sizing: border-box;
  }

  /* Hidden until it earns its place: a board whose columns all fit needs no map, and a strip of
     names under a board that shows those same names is clutter. `.overflowing` is measured; the
     narrow-pane rule below shows it unconditionally, because there one column fills the pane and
     every other column is off-screen by construction. */
  .rail {
    display: none;
    gap: 0.3rem;
    align-items: center;
    overflow-x: auto;
    padding: 0.35rem 1rem 0.5rem;
    border-top: 1px solid var(--column-border);
    flex: 0 0 auto;
  }
  .board-wrap.overflowing .rail {
    display: flex;
  }
  .rail-stop {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    flex: 0 0 auto;
    font: inherit;
    font-size: 0.78rem;
    color: var(--muted);
    background: var(--column-bg);
    border: 1px solid var(--column-border);
    border-radius: 999px;
    padding: 2px 8px;
    cursor: pointer;
    max-width: 9rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rail-stop.on {
    color: var(--text);
    border-color: var(--accent);
  }
  .rail-count {
    opacity: 0.65;
    font-variant-numeric: tabular-nums;
  }
  .column {
    flex: 0 0 17rem;
    background: var(--column-bg);
    border: 1px solid var(--column-border);
    border-radius: var(--radius-md);
    max-height: 100%;
    display: flex;
    flex-direction: column;
    transition:
      border-color var(--dur-fast) var(--ease),
      background var(--dur-fast) var(--ease);
  }
  .column.over {
    border-color: var(--accent);
    background: var(--column-over-bg);
  }
  .column-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.55rem 0.7rem;
    border-bottom: 1px solid var(--column-border);
    font-weight: 600;
    font-size: 0.85rem;
    cursor: grab;
    user-select: none;
  }
  .column-head:active {
    cursor: grabbing;
  }
  /* The label pill is colored by the value via a data attribute, so a theme can
     tint arbitrary enum values without this renderer knowing any of them. */
  .column-label {
    text-transform: none;
    color: var(--text);
  }
  .count {
    font-size: 0.75rem;
    color: var(--muted);
    background: var(--count-bg);
    border-radius: 999px;
    padding: 0.05rem 0.45rem;
  }
  .column-body {
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
    padding: 0.6rem;
    overflow-y: auto;
  }

  /* Phone: a column should fill the screen rather than showing 17rem of one and a
     sliver of the next — the board still scrolls sideways between columns, which is
     the gesture that already matches how a kanban board reads. */
  /* `@container`, not `@media`: "is this board narrow" is a question about the pane, and the
     answer differs per pane on the same screen. Same number as before, correct element. */
  @container pane (max-width: 40rem) {
    .board {
      padding: 0.5rem;
      gap: 0.5rem;
      scroll-snap-type: x mandatory;
    }
    .column {
      flex: 0 0 min(85vw, 22rem);
      scroll-snap-align: start;
    }
    /* One column fills the pane here, so every other column is off-screen whatever the
       measurement says — and a swipe is the only way to learn they exist. The rail is how you
       learn it without swiping. */
    .rail {
      display: flex;
      padding: 0.35rem 0.5rem 0.5rem;
    }
  }

  /* A coarse pointer cannot drag a column header (the element adapter is HTML5 drag),
     so stop advertising a gesture that does nothing there. Cards get the move menu;
     column order stays a desktop affordance, and it is a per-device localStorage
     preference anyway. */
  @media (pointer: coarse) {
    .column-head {
      cursor: default;
      padding: 0.75rem 0.7rem;
    }
  }
</style>
