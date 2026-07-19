<script lang="ts">
  import { draggable } from '@atlaskit/pragmatic-drag-and-drop/element/adapter';
  import StatusChip from '../lib/StatusChip.svelte';
  import VaultBadge from '../lib/VaultBadge.svelte';
  import EditedBy from '../lib/EditedBy.svelte';
  import { lastEditFor } from '../lib/activity.svelte';
  import { formatStamp } from '../lib/stamp';
  import type { ObjectMeta } from '../lib/types';

  // `statuses`/`onstatus` are optional: a view that doesn't offer status
  // rotation just omits them and the chip disappears.
  //
  // `columns`/`onmoveto` are the **touch path**. Dragging a card is HTML5 drag
  // (`@atlaskit/pragmatic-drag-and-drop`'s element adapter), which does not fire on touch at
  // all — and the package ships no pointer adapter to swap in, so on a phone the board is
  // simply not operable without this. Given a column list, the card offers a tap-to-move
  // menu that reuses the exact same write path a drop takes.
  //
  // Nothing here knows what the columns *mean*: `label` is whatever the grouped property's
  // values happen to be, so this stays as generic as the drag it stands in for.
  let { card, onopen, statuses = [], onstatus, columns = [], column = '', onmoveto }: {
    card: ObjectMeta;
    onopen: (id: string) => void;
    statuses?: string[];
    onstatus?: (id: string, value: string | null) => void;
    /** The columns this card could move to — `{value,label}` straight from the board. */
    columns?: { value: string; label: string }[];
    /** The column it is in now, so the menu can mark it and skip it. */
    column?: string;
    onmoveto?: (id: string, value: string) => void;
  } = $props();
  let menuOpen = $state(false);
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

  // The card itself is a button that opens the note, so every control inside it has to stop
  // the event climbing — otherwise moving a card also opens it.
  function swallow(e: Event) {
    e.stopPropagation();
  }

  function moveTo(value: string) {
    menuOpen = false;
    onmoveto?.(card.id, value);
  }

  const canMove = $derived(!!onmoveto && columns.length > 1);
</script>

<div
  class="card"
  class:dragging
  use:drag
  data-card-id={card.id}
  data-type={card.type}
  role="button"
  tabindex="0"
  onclick={open}
  onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && (e.preventDefault(), open())}
>
  <p class="preview">{card.title ?? card.preview ?? card.id}</p>
  <footer class="meta">
    {#if onstatus}
      <StatusChip
        status={card.status}
        {statuses}
        onchange={(next) => onstatus?.(card.id, next)}
      />
    {/if}
    {#if card.due}
      <span class="due" class:hard={card.hard} title={card.due}>{formatStamp(card.due)}</span>
    {/if}
    {#each card.tags as tag (tag)}
      <span class="tag">{tag}</span>
    {/each}
    <!-- Who last touched this, from git. Quiet, next to the tags; nothing when git is silent. -->
    <EditedBy edit={lastEditFor(card.id)} />
    <!-- Which audience this note belongs to — a name-coloured badge, the same everywhere.
         Derived from where the file lives, never from what it says, so it tells the truth
         about who can see it. Pushed to the trailing edge of the meta row. -->
    {#if card.vault}
      <span class="vault-cell"><VaultBadge vault={card.vault} /></span>
    {/if}
  </footer>

  {#if canMove}
    <!-- Touch's stand-in for a drag. Hidden from the pointer/keyboard path only by being
         small and quiet — it is a real button, so it works everywhere, which is what makes
         it testable without a phone. -->
    <button
      class="move"
      aria-haspopup="true"
      aria-expanded={menuOpen}
      aria-label="Move this card to another column"
      onclick={(e) => (swallow(e), (menuOpen = !menuOpen))}
      onkeydown={swallow}
    >⇄</button>
  {/if}
  {#if menuOpen}
    <!-- Plain buttons in a labelled group rather than a listbox: a real button is already
         focusable, keyboard-operable and announced correctly, where a `listbox` role would
         oblige us to hand-roll focus management to say the same thing. -->
    <div class="move-menu" role="group" aria-label="Move to column">
      {#each columns as col (col.value)}
        <button
          class="move-option"
          aria-current={col.value === column}
          disabled={col.value === column}
          onclick={(e) => (swallow(e), moveTo(col.value))}
        >{col.label}</button>
      {/each}
      <button
        class="move-option cancel"
        onclick={(e) => (swallow(e), (menuOpen = false))}
      >Cancel</button>
    </div>
  {/if}
</div>

<style>
  .card {
    background: var(--card-bg);
    border: 1px solid var(--card-border);
    border-radius: var(--radius-md);
    padding: 0.6rem 0.7rem;
    box-shadow: var(--shadow-sm);
    cursor: grab;
    position: relative; /* anchors the move button and its menu */
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
  /* Push the vault badge to the trailing edge — a tag is something you chose, a vault is
     who can see this, so they sit apart. The badge's own colour/shape lives in VaultBadge. */
  .vault-cell {
    margin-left: auto;
    display: inline-flex;
  }

  /* The touch stand-in for dragging. Quiet on a desktop — where the drag works — and a
     proper 44px target on a phone, where it is the only way to move a card at all. */
  .move {
    position: absolute;
    top: 0.25rem;
    right: 0.25rem;
    border: 1px solid transparent;
    background: none;
    color: var(--muted);
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-size: 0.9rem;
    line-height: 1;
    padding: 0.2rem 0.35rem;
    opacity: 0;
    transition: opacity var(--dur-fast) var(--ease);
  }
  .card:hover .move,
  .move:focus-visible {
    opacity: 1;
  }
  .move:hover,
  .move:focus-visible {
    color: var(--text);
    border-color: var(--card-border);
  }

  .move-menu {
    position: absolute;
    top: 1.9rem;
    right: 0.25rem;
    z-index: 10;
    display: flex;
    flex-direction: column;
    min-width: 9rem;
    background: var(--card-bg);
    border: 1px solid var(--card-border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-md, var(--shadow-sm));
    overflow: hidden;
  }
  .move-option {
    border: none;
    background: none;
    color: var(--text);
    text-align: left;
    padding: 0.5rem 0.7rem;
    font: inherit;
    font-size: 0.8rem;
    cursor: pointer;
  }
  .move-option:hover:not(:disabled) {
    background: var(--column-over-bg);
  }
  .move-option:disabled {
    color: var(--muted);
    cursor: default;
  }
  .move-option.cancel {
    border-top: 1px solid var(--card-border);
    color: var(--muted);
  }

  /* Phone: no hover to reveal anything, and a finger is not a mouse. The move button is
     always visible and big enough to hit, because without it the board is inert on touch. */
  /* Split deliberately. `pointer: coarse` is a *capability* of the input device and is a media
     query for the right reason — a tablet is wide and still has no mouse. The width half is a
     question about this card's container, so it moved to `@container`. They used to be OR'd
     into one rule, which meant a narrow pane on a mouse-driven desktop got touch affordances
     it did not need, and a wide pane on a touch screen missed them. */
  @media (pointer: coarse) {
    .move {
      opacity: 1;
      padding: 0.55rem 0.7rem;
      font-size: 1rem;
    }
    .move-menu {
      top: 2.6rem;
    }
    .move-option {
      padding: 0.75rem 0.8rem;
      font-size: 0.9rem;
    }
  }
  @container pane (max-width: 40rem) {
    .move {
      opacity: 1;
      padding: 0.55rem 0.7rem;
      font-size: 1rem;
    }
    .move-menu {
      top: 2.6rem;
    }
    .move-option {
      padding: 0.75rem 0.8rem;
      font-size: 0.9rem;
    }
  }
</style>
