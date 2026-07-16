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
          <Card {card} {onopen} {statuses} {onstatus} />
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
</style>
