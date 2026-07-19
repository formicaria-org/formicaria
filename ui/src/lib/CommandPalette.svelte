<script lang="ts">
  import { untrack } from 'svelte';
  interface Command {
    label: string;
    run: () => void;
    /** Which heading this sits under. Commands arrive already ordered by group; the palette
     *  only draws the dividers. */
    group?: string;
  }
  // `initial` lets a caller open the palette already scoped — the toolbar's "+" opens it on
  // "New", "+ view" on "Open". That is why this app has no dropdown menus: the palette is the
  // menu, it escapes the topbar's `overflow` clipping by being an overlay, and it is a far
  // better target on a phone than a 200px popover. See `Pane.svelte`'s "a rotator, not a
  // dropdown" for the same instinct applied elsewhere.
  let {
    commands,
    onclose,
    initial = '',
  }: { commands: Command[]; onclose: () => void; initial?: string } = $props();

  // Read once, at construction, and deliberately so: the palette is inside an `{#if}`, so it
  // is rebuilt every time it opens and `initial` is always the value the opener just set.
  // `untrack` says that out loud — without it Svelte warns that a later change to the prop
  // would not be picked up, which is true and is exactly the behaviour wanted.
  let query = $state(untrack(() => initial));
  let active = $state(0);
  let inputEl = $state<HTMLInputElement | undefined>(undefined);

  let filtered = $derived(
    query.trim()
      ? commands.filter((c) => c.label.toLowerCase().includes(query.trim().toLowerCase()))
      : commands,
  );

  $effect(() => {
    inputEl?.focus();
  });
  $effect(() => {
    if (active >= filtered.length) active = 0;
  });

  function choose(c: Command | undefined) {
    if (!c) return;
    onclose();
    c.run();
  }
  function onKey(e: KeyboardEvent) {
    const n = filtered.length;
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      active = n ? (active + 1) % n : 0;
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      active = n ? (active - 1 + n) % n : 0;
    } else if (e.key === 'Enter') {
      e.preventDefault();
      choose(filtered[active]);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      onclose();
    }
  }
</script>

<div class="palette-overlay">
  <button class="palette-backdrop" aria-label="close command palette" onclick={onclose}></button>
  <div class="palette" role="dialog" aria-label="command palette">
    <input
      bind:this={inputEl}
      bind:value={query}
      onkeydown={onKey}
      placeholder="Type a command…"
      aria-label="command"
      spellcheck="false"
    />
    <ul role="listbox">
      {#each filtered as c, i (c.label)}
        {#if c.group && c.group !== filtered[i - 1]?.group}
          <li class="group" role="presentation">{c.group}</li>
        {/if}
        <li
          role="option"
          aria-selected={i === active}
          class:active={i === active}
          onmousedown={(e) => {
            e.preventDefault();
            choose(c);
          }}
        >
          {c.label}
        </li>
      {/each}
      {#if filtered.length === 0}<li class="none">No matching command</li>{/if}
    </ul>
  </div>
</div>

<style>
  .palette-overlay {
    position: fixed;
    inset: 0;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 12vh;
    z-index: 80;
  }
  .palette-backdrop {
    position: fixed;
    inset: 0;
    border: none;
    background: rgb(0 0 0 / 0.45);
    cursor: default;
  }
  .palette {
    position: relative;
    width: min(34rem, 92vw);
    background: var(--surface-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
    overflow: hidden;
  }
  .palette input {
    width: 100%;
    box-sizing: border-box;
    padding: var(--space-3) var(--space-4);
    border: none;
    border-bottom: 1px solid var(--border);
    background: transparent;
    color: var(--text);
    font-size: var(--text-md);
  }
  .palette input:focus {
    outline: none;
  }
  ul {
    margin: 0;
    padding: var(--space-1);
    list-style: none;
    max-height: 20rem;
    overflow-y: auto;
  }
  li {
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    font-size: var(--text-sm);
    color: var(--text);
    cursor: pointer;
  }
  /* A heading, not a target: it is skipped by the arrow keys because it is not in `filtered`
     as a command — it is drawn between them. */
  li.group {
    padding: var(--space-2) var(--space-3) 2px;
    font-size: var(--text-xs);
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-muted);
    cursor: default;
  }
  li.group:hover {
    background: none;
  }
  li.active {
    background: var(--accent-subtle);
  }
  li.none {
    color: var(--text-muted);
    cursor: default;
  }
</style>
