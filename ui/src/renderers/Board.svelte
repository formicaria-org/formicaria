<script lang="ts">
  import { dropTargetForElements } from '@atlaskit/pragmatic-drag-and-drop/element/adapter';
  import Card from './Card.svelte';
  import type { Board } from '../lib/types';

  let { board, onmove, onopen }: {
    board: Board;
    onmove: (id: string, value: string) => void;
    onopen: (id: string) => void;
  } = $props();

  let over = $state<string | null>(null);

  // Each column is a drop target. Its identity is the grouped property's value —
  // an opaque string this renderer never inspects. On drop it hands the card id
  // and that value to the parent, which calls set_property(id, groupBy, value).
  // There is no reference here to any particular property or status name.
  function column(node: HTMLElement, value: string) {
    let current = value;
    const cleanup = dropTargetForElements({
      element: node,
      getData: () => ({ value: current }),
      onDragEnter: () => (over = current),
      onDragLeave: () => {
        if (over === current) over = null;
      },
      onDrop: ({ source }) => {
        over = null;
        const id = source.data.id;
        if (typeof id === 'string') onmove(id, current);
      },
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
      <header class="column-head" data-value={col.value}>
        <span class="column-label">{col.label}</span>
        <span class="count">{col.cards.length}</span>
      </header>
      <div class="column-body">
        {#each col.cards as card (card.id)}
          <Card {card} {onopen} />
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
