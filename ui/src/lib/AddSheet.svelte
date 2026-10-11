<script lang="ts">
  // What a group's add control offers when there is more than one thing to make: a plain note, and
  // the templates that name the group (`decisions.md#ui`, 2026-10-12).
  import Icon from './Icon.svelte';

  interface Props {
    group: string;
    choices: { key: string; label: string }[];
    onpick: (key: string) => void;
    onclose: () => void;
  }
  let { group, choices, onpick, onclose }: Props = $props();
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<div class="add-overlay">
  <button type="button" class="scrim" onclick={onclose} aria-label="close" tabindex="-1"></button>
  <div class="sheet" role="dialog" aria-modal="true" aria-label={`add to ${group}`}>
    <h2>New in {group}</h2>
    <ul>
      {#each choices as c (c.key)}
        <li>
          <button type="button" onclick={() => onpick(c.key)}>
            <Icon name="plus" size={18} />
            {c.label}
          </button>
        </li>
      {/each}
    </ul>
  </div>
</div>

<style>
  .add-overlay {
    position: fixed;
    inset: 0;
    z-index: 80;
    display: flex;
    justify-content: center;
    align-items: flex-end;
    box-sizing: border-box;
    height: var(--app-h);
    padding-top: calc(var(--overlay-inset) + var(--safe-top));
  }
  .scrim {
    position: fixed;
    inset: 0;
    border: 0;
    background: rgb(0 0 0 / 0.45);
    cursor: default;
  }
  .sheet {
    position: relative;
    width: min(34rem, 100%);
    max-height: 100%;
    overflow-y: auto;
    overscroll-behavior: contain;
    box-sizing: border-box;
    padding: var(--space-4) var(--space-4) max(var(--safe-bottom), var(--space-4));
    background: var(--surface-elevated);
    border-radius: var(--radius-lg) var(--radius-lg) 0 0;
    box-shadow: var(--shadow-lg);
  }
  h2 {
    margin: 0 0 var(--space-3);
    font-size: 1.05rem;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: var(--space-2);
  }
  button:not(.scrim) {
    width: 100%;
    min-height: 3rem;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding-inline: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
</style>
