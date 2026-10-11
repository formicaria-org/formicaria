<script lang="ts">
  // A row of bars, one per day, with the target drawn across them. Each bar is a real button, so a
  // day can be chosen by touch or by keyboard.
  import { dur } from './motion';

  interface Bar {
    key: string;
    value: number;
    /** What a screen reader says for this bar. */
    label: string;
    /** A short mark under the bar. */
    tick?: string;
    /** The target in force for this bar. Drawn across the column, so a changed target is a step. */
    target?: number;
  }
  interface Props {
    bars: Bar[];
    target?: number;
    selected?: string;
    height?: number;
    onpick?: (key: string) => void;
  }
  let { bars, target = 0, selected, height = 140, onpick }: Props = $props();

  const targetOf = (b: Bar) => b.target ?? target;
  const top = $derived(Math.max(...bars.map((b) => Math.max(b.value, targetOf(b))), 1) * 1.1);
  let grown = $state(false);
  $effect(() => {
    // One frame at zero height, then the real one, so the bars rise once when they arrive.
    const id = requestAnimationFrame(() => (grown = true));
    return () => cancelAnimationFrame(id);
  });
</script>

<div class="bars" style="height:{height}px; --rise:{dur(420)}ms">
  {#each bars as b (b.key)}
    <button
      type="button"
      class="col"
      class:on={b.key === selected}
      aria-label={b.label}
      aria-pressed={b.key === selected}
      onclick={() => onpick?.(b.key)}
    >
      <span class="well">
        {#if targetOf(b) > 0}
          <span class="target" style="bottom:{(targetOf(b) / top) * 100}%"></span>
        {/if}
        <span class="bar" style="height:{grown ? (b.value / top) * 100 : 0}%"></span>
      </span>
      <span class="tick">{b.tick ?? ''}</span>
    </button>
  {/each}
</div>

<style>
  .bars {
    position: relative;
    display: flex;
    align-items: stretch;
    gap: 3px;
  }
  .col {
    flex: 1 1 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 0;
    border: 0;
    background: none;
    color: var(--text-subtle);
    cursor: pointer;
  }
  .well {
    position: relative;
    flex: 1;
    display: flex;
    align-items: flex-end;
  }
  .bar {
    width: 100%;
    min-height: 2px;
    border-radius: var(--radius-sm) var(--radius-sm) 0 0;
    background: var(--accent);
    opacity: 0.7;
    transition: height var(--rise) var(--ease);
  }
  .col.on .bar,
  .col:hover .bar {
    opacity: 1;
  }
  .col.on .tick {
    color: var(--text);
    font-weight: 600;
  }
  .tick {
    height: 1rem;
    font-size: var(--text-xs);
    line-height: 1rem;
    text-align: center;
  }
  .target {
    position: absolute;
    left: -2px;
    right: -2px;
    border-top: 1px dashed var(--border-strong);
    pointer-events: none;
  }
</style>
