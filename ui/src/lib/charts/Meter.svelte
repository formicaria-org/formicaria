<script lang="ts">
  // One bar against a target.
  import { Tween } from 'svelte/motion';
  import { tween } from './motion';

  interface Props {
    value: number;
    target: number;
    /** Which of the three series tints to draw in. */
    tint?: 'a' | 'b' | 'c';
    label: string;
  }
  let { value, target, tint = 'a', label }: Props = $props();

  const shown = new Tween(0, tween());
  $effect(() => {
    shown.target = target > 0 ? Math.min(Math.max(value / target, 0), 1) : 0;
  });
</script>

<div class="meter" role="img" aria-label={label}>
  <div class="fill tint-{tint}" style="width:{shown.current * 100}%"></div>
</div>

<style>
  .meter {
    height: 0.5rem;
    border-radius: var(--radius-pill);
    background: var(--surface-hover);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    border-radius: var(--radius-pill);
  }
  .tint-a {
    background: var(--tint-a);
  }
  .tint-b {
    background: var(--tint-b);
  }
  .tint-c {
    background: var(--tint-c);
  }
</style>
