<script lang="ts">
  // **The Library's shelves** (`decisions.md#ui`, 2026-10-18, *Home is the Library's shelves*). The
  // Board's grouping, drawn as a bookcase: each group is a shelf, each note a book lying on its
  // side, spines one above another so every title reads left to right. Dropping a book on another
  // shelf sets the grouped property — exactly what moving a board card does.
  //
  // **No shelf is named here.** Every label comes with the view (`library-shelves.view`): which
  // shelves exist is the notes' own `shelf` property, and what the notes on none are called is the
  // view's `rest`. The renderer only knows a group with an empty value is that rest.
  import {
    draggable,
    dropTargetForElements,
  } from '@atlaskit/pragmatic-drag-and-drop/element/adapter';
  import { hashHue } from '../lib/vaultColor';
  import EmptyState from '../lib/EmptyState.svelte';
  import type { Board } from '../lib/types';

  let {
    board,
    onmove,
    onopen,
  }: {
    board: Board;
    /** Put a book on a shelf: set the grouped property to `value`. */
    onmove: (id: string, value: string) => void;
    onopen: (id: string) => void;
  } = $props();

  /// The rest can be every note in the notebook; a shelf shows its newest this many, and says so.
  const SHOWN = 30;
  let over = $state<string | null>(null);

  function shelf(node: HTMLElement, value: string) {
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
    return { update: (v: string) => (current = v), destroy: cleanup };
  }

  function book(node: HTMLElement, id: string) {
    let current = id;
    const cleanup = draggable({ element: node, getInitialData: () => ({ id: current }) });
    return { update: (v: string) => (current = v), destroy: cleanup };
  }

  /// A book's width varies a little with its title, as real spines do — only so the stack reads
  /// as books and not as a list.
  const width = (title: string) => 88 + (hashHue(title) % 13);
</script>

{#if board.columns.length === 0}
  <EmptyState
    icon="timeline"
    title="No notes on these shelves yet"
    hint="Press + to write one; it lands here."
  />
{:else}
  <div class="case">
    {#each board.columns as s (s.value)}
      <section class="shelf" class:over={over === s.value} aria-label={s.label} use:shelf={s.value}>
        <h3>{s.label} <span class="count">{s.cards.length}</span></h3>
        <ul class="stack">
          {#each s.cards.slice(0, SHOWN) as c (c.id)}
            {@const title = c.title ?? c.preview ?? ''}
            <li>
              <button
                class="book"
                style="--hue: {hashHue(title)}; width: {width(title)}%"
                onclick={() => onopen(c.id)}
                use:book={c.id}
                {title}>{title}</button
              >
            </li>
          {/each}
        </ul>
        {#if s.cards.length > SHOWN}
          <p class="more">{s.cards.length - SHOWN} more — every note is in Timeline.</p>
        {/if}
        <div class="ledge" aria-hidden="true"></div>
      </section>
    {/each}
  </div>
{/if}

<style>
  .case {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, 18rem), 1fr));
    gap: 1.5rem;
    padding: 1rem;
    align-items: end;
  }
  .shelf {
    display: flex;
    flex-direction: column;
    min-width: 0;
    border-radius: var(--radius-2, 6px);
    padding: 0.25rem;
  }
  .shelf.over {
    background: var(--surface-hover);
  }
  h3 {
    margin: 0 0 0.5rem;
    font-size: 0.95rem;
  }
  .count {
    color: var(--muted);
    font-weight: normal;
  }
  .stack {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .book {
    display: block;
    max-width: 100%;
    min-height: 2.4rem;
    padding: 0 0.75rem;
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    border: 1px solid hsl(var(--hue) 35% 40% / 0.6);
    border-radius: 3px;
    background: linear-gradient(to bottom, hsl(var(--hue) 45% 62%), hsl(var(--hue) 45% 54%));
    color: hsl(var(--hue) 40% 12%);
    font-weight: 600;
    cursor: pointer;
  }
  .book:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  .ledge {
    height: 0.4rem;
    margin-top: 2px;
    border-radius: 2px;
    background: var(--border);
  }
  .more {
    margin: 0.4rem 0 0;
    color: var(--muted);
    font-size: 0.85rem;
  }
</style>
