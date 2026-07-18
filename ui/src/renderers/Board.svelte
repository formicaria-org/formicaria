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

<div class="board">
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

<style>
  .board {
    display: flex;
    gap: 0.85rem;
    align-items: flex-start;
    overflow-x: auto;
    padding: 1rem;
    height: 100%;
    box-sizing: border-box;
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
  @media (max-width: 40rem) {
    .board {
      padding: 0.5rem;
      gap: 0.5rem;
      scroll-snap-type: x mandatory;
    }
    .column {
      flex: 0 0 min(85vw, 22rem);
      scroll-snap-align: start;
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
