<script lang="ts">
  // The Library's way in by tag: every tag on the notes shown, with how many carry it, most used
  // first. A tap keeps the notes with that tag; a second tap lets them all back. Counted here
  // from the rows already fetched (`decisions.md#ui`, 2026-10-12) — no command, no second list.
  import Meter from '../charts/Meter.svelte';

  interface Props {
    tags: string[][];
    chosen: string | null;
    onpick: (tag: string | null) => void;
  }
  let { tags, chosen, onpick }: Props = $props();

  const counts = $derived.by(() => {
    const n = new Map<string, number>();
    for (const list of tags) for (const t of new Set(list)) n.set(t, (n.get(t) ?? 0) + 1);
    return [...n.entries()].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
  });
  const most = $derived(counts[0]?.[1] ?? 0);
</script>

{#if counts.length}
  <ul class="tag-chips" aria-label="tags">
    {#each counts as [tag, n] (tag)}
      <li>
        <button
          type="button"
          class:on={tag === chosen}
          aria-pressed={tag === chosen}
          onclick={() => onpick(tag === chosen ? null : tag)}
        >
          <span class="row"><span class="tag">{tag}</span><span class="n">{n}</span></span>
          <span aria-hidden="true"><Meter value={n} target={most} label={tag} /></span>
        </button>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .tag-chips {
    list-style: none;
    margin: 0;
    padding: var(--space-2) var(--space-3);
    display: flex;
    gap: var(--space-2);
    overflow-x: auto;
    scroll-snap-type: x proximity;
    scrollbar-width: none;
  }
  li {
    flex: none;
    scroll-snap-align: start;
  }
  button {
    min-width: 6.5rem;
    min-height: 2.75rem;
    display: grid;
    gap: var(--space-1);
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  button.on {
    border-color: var(--accent);
    background: var(--accent-subtle);
  }
  .row {
    display: flex;
    justify-content: space-between;
    gap: var(--space-3);
  }
  .tag {
    font-weight: 600;
  }
  .n {
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }
</style>
