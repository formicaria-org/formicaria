<script lang="ts">
  // The bar a group of views wears on a phone: a way to everything else, the group's views, and one
  // raised control to add (`decisions.md#ui`, 2026-10-11 — the owner chose that a group's own bar
  // replaces the app's while the group is open).
  import Icon from './Icon.svelte';
  import type { ViewGroup } from './types';

  interface Props {
    group: ViewGroup;
    current: string | null;
    onpick: (view: string) => void;
    onadd: () => void;
    /** Show the app's own bar again — search, windows, backing up, settings. */
    onmore: () => void;
  }
  let { group, current, onpick, onadd, onmore }: Props = $props();
  const half = $derived(Math.ceil(group.views.length / 2));
</script>

<nav class="group-bar" aria-label={group.name}>
  <button type="button" class="side" onclick={onmore} aria-label="everything else">
    <Icon name="more" size={20} />
  </button>
  {#snippet tab(v: string)}
    <button
      type="button"
      class="tab"
      class:on={v === current}
      aria-current={v === current ? 'page' : undefined}
      onclick={() => onpick(v)}>{v}</button
    >
  {/snippet}
  <!-- The add control sits in the middle of the views, where a thumb rests. -->
  {#each group.views.slice(0, half) as v (v)}{@render tab(v)}{/each}
  <button type="button" class="add" onclick={onadd} aria-label={`add to ${group.name}`}>
    <Icon name="plus" size={24} />
  </button>
  {#each group.views.slice(half) as v (v)}{@render tab(v)}{/each}
  <!-- Balances the control on the left, so the add control is in the true middle. -->
  <span class="side" aria-hidden="true"></span>
</nav>

<style>
  .group-bar {
    flex: 1;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
  }
  button {
    font: inherit;
    color: inherit;
    cursor: pointer;
  }
  .side {
    flex: none;
    width: 2.75rem;
    height: 2.75rem;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: var(--radius-pill);
    background: none;
    color: var(--text-muted);
  }
  .tab {
    flex: 1;
    min-height: 2.75rem;
    border: 0;
    border-radius: var(--radius-md);
    background: none;
    color: var(--text-muted);
  }
  .tab.on {
    color: var(--accent);
    font-weight: 600;
  }
  .add {
    flex: none;
    width: 3.25rem;
    height: 3.25rem;
    margin-top: -0.9rem;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: var(--radius-pill);
    background: var(--accent);
    color: var(--accent-contrast);
    box-shadow: var(--shadow-md);
    transition: transform var(--dur-fast) var(--ease);
  }
  .add:active {
    transform: scale(0.92);
  }
</style>
