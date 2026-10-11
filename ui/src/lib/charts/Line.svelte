<script lang="ts">
  // A line through a run of days — for a value that is measured, not added up (a weight), and for
  // spans too long for a bar a day. A day with nothing is a gap in the points, never a zero.
  import { dur } from './motion';

  interface Point {
    key: string;
    /** `null` where the day has no value. */
    value: number | null;
    /** The target in force that day, if any: drawn as steps. */
    target?: number;
  }
  interface Props {
    points: Point[];
    height?: number;
    /** What a screen reader says for the whole chart. */
    label: string;
  }
  let { points, height = 140, label }: Props = $props();

  const W = 300;
  const PAD = 8;
  const have = $derived(points.filter((p) => p.value !== null) as (Point & { value: number })[]);
  const lo = $derived(
    Math.min(...have.map((p) => p.value), ...points.map((p) => p.target ?? Infinity)),
  );
  const hi = $derived(
    Math.max(...have.map((p) => p.value), ...points.map((p) => p.target ?? -Infinity)),
  );
  /// A flat line still needs a band to sit in.
  const span = $derived(hi - lo || Math.max(Math.abs(hi), 1) * 0.1);
  const x = (i: number) =>
    points.length < 2 ? W / 2 : PAD + (i / (points.length - 1)) * (W - 2 * PAD);
  const y = (v: number) => PAD + (1 - (v - lo) / span) * (height - 2 * PAD);

  const path = $derived(
    points
      .map((p, i) => (p.value === null ? null : `${x(i).toFixed(1)},${y(p.value).toFixed(1)}`))
      .filter(Boolean)
      .join(' '),
  );
  const steps = $derived.by(() => {
    let d = '';
    let pen = false;
    points.forEach((p, i) => {
      if (p.target === undefined || p.target <= 0) {
        pen = false;
        return;
      }
      d += `${pen ? 'L' : 'M'}${x(i).toFixed(1)},${y(p.target).toFixed(1)} `;
      const next = points[i + 1];
      if (next?.target !== undefined && next.target > 0)
        d += `L${x(i + 1).toFixed(1)},${y(p.target).toFixed(1)} `;
      pen = true;
    });
    return d;
  });
  const last = $derived(have.at(-1));
  const lastAt = $derived(last ? points.indexOf(last) : -1);
</script>

<!-- The drawing stretches to the width it is given, so the last value's dot is not part of it: a
     circle in a stretched drawing is an oval. It is placed over the drawing, by the same sums. -->
<div class="chart" style="height:{height}px" role="img" aria-label={label}>
  <svg viewBox="0 0 {W} {height}" preserveAspectRatio="none" aria-hidden="true">
    {#if steps}<path class="target" d={steps} />{/if}
    {#if have.length > 1}
      <polyline class="trace" points={path} />
    {/if}
  </svg>
  {#if last}
    <span
      class="dot"
      style="left:{(x(lastAt) / W) * 100}%; top:{(y(last.value) / height) * 100}%; --fade:{dur(
        400,
      )}ms"
    ></span>
  {/if}
</div>

<style>
  .chart {
    position: relative;
  }
  svg {
    display: block;
    width: 100%;
    height: 100%;
    overflow: visible;
  }
  .trace {
    fill: none;
    stroke: var(--accent);
    stroke-width: 2.5;
    stroke-linecap: round;
    stroke-linejoin: round;
    vector-effect: non-scaling-stroke;
  }
  .target {
    fill: none;
    stroke: var(--border-strong);
    stroke-width: 1;
    stroke-dasharray: 4 3;
    vector-effect: non-scaling-stroke;
  }
  .dot {
    position: absolute;
    width: 9px;
    height: 9px;
    margin: -4.5px 0 0 -4.5px;
    border-radius: var(--radius-pill);
    background: var(--accent);
    box-shadow: 0 0 0 3px var(--surface-elevated);
    animation: dot-in var(--fade) var(--ease);
  }
  @keyframes dot-in {
    from {
      opacity: 0;
      transform: scale(0.4);
    }
  }
</style>
