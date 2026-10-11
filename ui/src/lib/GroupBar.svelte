<script lang="ts">
  // The bar a group of views wears on a phone: a way to the other groups and everything else, the
  // group's views, and one raised control to add (`decisions.md#ui`, 2026-10-11 and 2026-10-12).
  import Icon from './Icon.svelte';

  interface Props {
    name: string;
    tabs: { key: string; label: string }[];
    current: string | null;
    onpick: (key: string) => void;
    onadd: () => void;
    /** The first control: the list of groups, or the app's own bar. */
    onmore: () => void;
    /** What that control is called. */
    more?: string;
  }
  let { name, tabs, current, onpick, onadd, onmore, more = 'everything else' }: Props = $props();
  const half = $derived(Math.ceil(tabs.length / 2));
</script>

<nav class="group-bar" aria-label={name}>
  <button type="button" class="side" onclick={onmore} aria-label={more}>
    <Icon name="more" size={20} />
  </button>
  {#snippet tab(t: { key: string; label: string })}
    <button
      type="button"
      class="tab"
      class:on={t.key === current}
      aria-current={t.key === current ? 'page' : undefined}
      onclick={() => onpick(t.key)}>{t.label}</button
    >
  {/snippet}
  <!-- The add control sits in the middle of the views, where a thumb rests. -->
  {#each tabs.slice(0, half) as t (t.key)}{@render tab(t)}{/each}
  <button type="button" class="add" onclick={onadd} aria-label={`add to ${name}`}>
    <Icon name="plus" size={24} />
  </button>
  {#each tabs.slice(half) as t (t.key)}{@render tab(t)}{/each}
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
