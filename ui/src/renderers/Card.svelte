<script lang="ts">
  import { draggable } from '@atlaskit/pragmatic-drag-and-drop/element/adapter';
  import type { ObjectMeta } from '../lib/types';

  let { card, onopen }: { card: ObjectMeta; onopen: (id: string) => void } = $props();
  let dragging = $state(false);
  // Suppress the click that trails a drag, so dropping a card never also opens it.
  let suppressClick = false;

  // A card is draggable; its id is the payload the column reads on drop.
  function drag(node: HTMLElement) {
    return {
      destroy: draggable({
        element: node,
        getInitialData: () => ({ id: card.id }),
        onDragStart: () => {
          dragging = true;
          suppressClick = true;
        },
        onDrop: () => {
          dragging = false;
          setTimeout(() => (suppressClick = false), 0);
        },
      }),
    };
  }

  function open() {
    if (!suppressClick) onopen(card.id);
  }
</script>

<div
  class="card"
  class:dragging
  use:drag
  data-type={card.type}
  role="button"
  tabindex="0"
  onclick={open}
  onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && (e.preventDefault(), open())}
>
  <p class="preview">{card.title ?? card.preview ?? card.id}</p>
  <footer class="meta">
    {#if card.due}
      <span class="due" class:hard={card.hard}>{card.due}</span>
    {/if}
    {#each card.tags as tag (tag)}
      <span class="tag">{tag}</span>
    {/each}
  </footer>
</div>

<style>
  .card {
    background: var(--card-bg);
    border: 1px solid var(--card-border);
    border-radius: var(--radius-md);
    padding: 0.6rem 0.7rem;
    box-shadow: var(--shadow-sm);
    cursor: grab;
    transition:
      box-shadow var(--dur-fast) var(--ease),
      border-color var(--dur-fast) var(--ease),
      transform var(--dur-fast) var(--ease);
  }
  .card:hover {
    box-shadow: var(--shadow-md);
    border-color: var(--border-strong);
  }
  .card.dragging {
    opacity: 0.4;
    transform: scale(0.98);
  }
  .preview {
    margin: 0;
    font-size: 0.9rem;
    line-height: 1.35;
    color: var(--text);
    overflow-wrap: anywhere;
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
    margin-top: 0.5rem;
    align-items: center;
  }
  .meta:empty {
    display: none;
  }
  .due {
    font-size: 0.72rem;
    padding: 0.05rem 0.4rem;
    border-radius: 999px;
    background: var(--due-bg);
    color: var(--due-fg);
  }
  .due.hard {
    background: var(--due-hard-bg);
    color: var(--due-hard-fg);
    font-weight: 600;
  }
  .tag {
    font-size: 0.72rem;
    padding: 0.05rem 0.4rem;
    border-radius: 999px;
    background: var(--tag-bg);
    color: var(--tag-fg);
  }
</style>
