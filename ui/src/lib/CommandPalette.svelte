<script lang="ts">
  interface Command {
    label: string;
    run: () => void;
  }
  let { commands, onclose }: { commands: Command[]; onclose: () => void } = $props();

  let query = $state('');
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
  li.active {
    background: var(--accent-subtle);
  }
  li.none {
    color: var(--text-muted);
    cursor: default;
  }
</style>
