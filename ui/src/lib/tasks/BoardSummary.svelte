<script lang="ts">
  // What heads the board in Tasks: how much of it has reached the last column, and how the rest is
  // spread. The columns and their order are the board's own, so no status is named here
  // (`decisions.md#ui`, 2026-10-12).
  import Ring from '../charts/Ring.svelte';
  import Meter from '../charts/Meter.svelte';

  interface Props {
    columns: { label: string; count: number }[];
  }
  let { columns }: Props = $props();

  const total = $derived(columns.reduce((n, c) => n + c.count, 0));
  const last = $derived(columns.at(-1));
  const TINTS = ['a', 'b', 'c'] as const;
</script>

{#if total > 0 && last}
  <section class="board-summary" aria-label="how the board stands">
    <Ring
      value={last.count}
      target={total}
      size={84}
      stroke={9}
      label={`${last.count} of ${total} in ${last.label}`}
    >
      <span class="big">{last.count}</span>
      <span class="of">of {total}</span>
    </Ring>
    <ul>
      {#each columns as c, i (c.label)}
        <li>
          <span class="row"><span>{c.label}</span><span class="n">{c.count}</span></span>
          <span aria-hidden="true">
            <Meter value={c.count} target={total} tint={TINTS[i % 3]} label={c.label} />
          </span>
        </li>
      {/each}
    </ul>
  </section>
{/if}

<style>
  .board-summary {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-3) var(--space-4);
  }
  .big {
    font-size: 1.4rem;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  .of {
    font-size: 0.75rem;
    color: var(--text-muted);
  }
  ul {
    flex: 1;
    min-width: 0;
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: var(--space-2);
  }
  .row {
    display: flex;
    justify-content: space-between;
    font-size: 0.85rem;
    margin-bottom: 2px;
  }
  .n {
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }
</style>
