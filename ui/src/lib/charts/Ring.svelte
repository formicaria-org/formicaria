<script lang="ts">
  // A ring that fills towards a target. Past the target it starts a second lap on top of the
  // first, in the same calm colour — going over is a fact, not an alarm.
  import type { Snippet } from 'svelte';
  import { Tween } from 'svelte/motion';
  import { tween } from './motion';

  interface Props {
    value: number;
    /** 0 or less means "no target": the ring is an empty track. */
    target: number;
    size?: number;
    stroke?: number;
    label: string;
    children?: Snippet;
  }
  let { value, target, size = 168, stroke = 14, label, children }: Props = $props();

  const part = $derived(target > 0 ? Math.max(0, value / target) : 0);
  const shown = new Tween(0, tween());
  $effect(() => {
    shown.target = part;
  });

  const r = $derived((size - stroke) / 2);
  const c = $derived(2 * Math.PI * r);
  const first = $derived(Math.min(shown.current, 1));
  const second = $derived(Math.min(Math.max(shown.current - 1, 0), 1));
</script>

<!-- With no label it is decoration inside something that already has a name. -->
<div
  class="ring"
  style="width:{size}px;height:{size}px"
  role={label ? 'img' : undefined}
  aria-label={label || undefined}
  aria-hidden={label ? undefined : true}
>
  <svg width={size} height={size} viewBox="0 0 {size} {size}" aria-hidden="true">
    <circle class="track" cx={size / 2} cy={size / 2} {r} stroke-width={stroke} />
    <!-- Nothing yet is no lap at all: a zero-length stroke with round ends still draws a dot. -->
    {#if first > 0}
      <circle
        class="lap"
        cx={size / 2}
        cy={size / 2}
        {r}
        stroke-width={stroke}
        stroke-dasharray="{first * c} {c}"
      />
    {/if}
    {#if second > 0}
      <circle
        class="lap over"
        cx={size / 2}
        cy={size / 2}
        {r}
        stroke-width={stroke * 0.5}
        stroke-dasharray="{second * c} {c}"
      />
    {/if}
  </svg>
  <div class="middle">{@render children?.()}</div>
</div>

<style>
  .ring {
    position: relative;
    flex: none;
  }
  svg {
    display: block;
    transform: rotate(-90deg);
  }
  circle {
    fill: none;
    stroke-linecap: round;
  }
  .track {
    stroke: var(--surface-hover);
  }
  .lap {
    stroke: var(--accent);
  }
  .lap.over {
    stroke: var(--accent-contrast);
    opacity: 0.85;
  }
  .middle {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    text-align: center;
  }
</style>
