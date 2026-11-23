<script lang="ts">
  // **The vault chooser** (the rail, 2026-11-23; the owner: *"where did the vaults go? How can I
  // select the ones to see?"*). With two or more vaults, the rail's top says which are shown and
  // opens a list to tick them. Hidden vaults' notes leave every section, Home, the search and the
  // counts (`shownVaults.svelte.ts`).
  import { onMount } from 'svelte';
  import { labelFor, refreshVaults, vaultList } from '../lib/vaults.svelte';
  import { hiddenVaults, toggleVault } from '../lib/shownVaults.svelte';
  import { popup } from '../lib/popup';

  let open = $state(false);
  const all = $derived(vaultList() ?? []);
  const hidden = $derived(hiddenVaults());
  const label = $derived(
    hidden.length ? `${all.length - hidden.length} of ${all.length} vaults` : 'All vaults',
  );
  onMount(() => {
    if (!vaultList()) void refreshVaults();
    const away = (e: MouseEvent) => {
      if (!(e.target as HTMLElement)?.closest?.('.vaults')) open = false;
    };
    addEventListener('click', away);
    return () => removeEventListener('click', away);
  });
</script>

{#if all.length > 1}
  <div class="vaults">
    <button
      type="button"
      class="choose"
      class:filtering={hidden.length > 0}
      aria-expanded={open}
      aria-haspopup="true"
      title="Choose the vaults to see"
      onclick={() => (open = !open)}>{label} ▾</button
    >
    {#if open}
      <div class="list" role="group" aria-label="vaults to see" use:popup={{ align: 'start' }}>
        {#each all as v (v.name)}
          {@const on = !hidden.includes(v.name)}
          <label
            ><input
              type="checkbox"
              checked={on}
              disabled={on && all.length - hidden.length <= 1}
              onchange={() => toggleVault(v.name)}
            />
            {labelFor(v.name)}</label
          >
        {/each}
        <p class="hint">
          Hidden vaults' notes are left out of every page here. Nothing is deleted.
        </p>
      </div>
    {/if}
  </div>
{/if}

<style>
  .vaults {
    position: relative;
  }
  .choose {
    width: 100%;
    text-align: left;
    font: inherit;
    font-weight: 650;
    padding: var(--space-1) var(--space-2);
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text);
    cursor: pointer;
  }
  .choose:hover,
  .choose.filtering {
    border-color: var(--border);
  }
  .choose.filtering {
    color: var(--accent);
  }
  .list {
    position: absolute;
    z-index: 30;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    padding: var(--space-2);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-md, 0 4px 16px rgb(0 0 0 / 0.15));
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  label {
    display: flex;
    gap: var(--space-2);
    align-items: center;
    color: var(--text);
  }
  .hint {
    margin: var(--space-1) 0 0;
    font-size: var(--text-xs);
    color: var(--text-muted);
  }
</style>
