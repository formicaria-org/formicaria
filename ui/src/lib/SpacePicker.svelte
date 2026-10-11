<script lang="ts">
  // The list a phone's bar opens from its first control: the groups, then everything else the
  // app's own bar holds (`decisions.md#ui`, 2026-10-12). One tap to any group from anywhere.
  import Icon from './Icon.svelte';
  import type { Space } from './spaces.svelte';

  interface Props {
    spaces: Space[];
    current: string | null;
    onpick: (name: string) => void;
    /** Show the app's own bar: search, windows, backing up, settings. */
    onrest: () => void;
    onclose: () => void;
  }
  let { spaces, current, onpick, onrest, onclose }: Props = $props();
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<div class="picker-overlay">
  <button type="button" class="scrim" onclick={onclose} aria-label="close" tabindex="-1"></button>
  <div class="sheet" role="dialog" aria-modal="true" aria-label="groups">
    <ul>
      {#each spaces as s (s.name)}
        <li>
          <button
            type="button"
            class="space"
            class:on={s.name === current}
            aria-current={s.name === current ? 'page' : undefined}
            onclick={() => onpick(s.name)}
          >
            <span class="badge"><Icon name={s.icon} size={20} /></span>
            <span class="name">{s.name}</span>
            <span class="what">{s.tabs.map((t) => t.label).join(' · ')}</span>
          </button>
        </li>
      {/each}
    </ul>
    <button type="button" class="rest" onclick={onrest}>
      <Icon name="more" size={18} />
      Everything else
    </button>
  </div>
</div>

<style>
  .picker-overlay {
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
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    background: var(--surface-elevated);
    border-radius: var(--radius-lg) var(--radius-lg) 0 0;
    box-shadow: var(--shadow-lg);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-3);
  }
  button {
    font: inherit;
    color: inherit;
    cursor: pointer;
  }
  .space {
    width: 100%;
    min-height: 5.5rem;
    display: grid;
    grid-template-columns: auto 1fr;
    grid-template-rows: auto auto;
    column-gap: var(--space-3);
    align-items: center;
    padding: var(--space-3);
    text-align: left;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }
  .space.on {
    border-color: var(--accent);
    background: var(--accent-subtle);
  }
  .badge {
    grid-row: 1 / 3;
    width: 2.5rem;
    height: 2.5rem;
    display: grid;
    place-items: center;
    border-radius: var(--radius-pill);
    background: var(--surface-hover);
    color: var(--accent);
  }
  .name {
    font-weight: 700;
    font-size: 1.05rem;
  }
  .what {
    color: var(--text-muted);
    font-size: 0.85rem;
  }
  .rest {
    min-height: 2.75rem;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding-inline: var(--space-3);
    border: 0;
    border-radius: var(--radius-md);
    background: none;
    color: var(--text-muted);
  }
</style>
