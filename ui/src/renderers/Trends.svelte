<script lang="ts">
  // A run of days: **the one value picked from the menu** drawn against its target, the average of
  // each of the others below it, and how many of the days have anything logged at all — a week's average over three logged days
  // is not a week's average, and the screen says so.
  //
  // A value that is added up (energy) is a bar a day; one that is measured (a weight) is a line,
  // and so is any span too long for a bar a day. The target is drawn per day, so the day it
  // changed shows as a step.
  //
  // Generic, like `Totals`: everything it names comes from the view's definition.
  import Bars from '../lib/charts/Bars.svelte';
  import Line from '../lib/charts/Line.svelte';
  import Meter from '../lib/charts/Meter.svelte';
  import EmptyState from '../lib/EmptyState.svelte';
  import Icon from '../lib/Icon.svelte';
  import { show, showWith, dayLabel, dayLetter, dayNumber } from '../lib/units';
  import type { DayTotals, Metric, TotalsDef } from '../lib/types';

  interface Props {
    def: TotalsDef;
    days: DayTotals[];
    /** How many days are shown — one of `spans`. */
    span: number;
    spans?: { days: number; label: string }[];
    onspan: (days: number) => void;
    onday: (day: string) => void;
    /** Add an entry from here — for a view with no day screen of its own to add from. */
    onadd?: () => void;
    /** Write down a value that is kept in a log of its own — a weight among the food trends. */
    onmeasure?: (m: Metric) => void;
  }
  let {
    def,
    days,
    span,
    spans = [
      { days: 7, label: 'Week' },
      { days: 28, label: '4 weeks' },
      { days: 84, label: '12 weeks' },
    ],
    onspan,
    onday,
    onadd,
    onmeasure,
  }: Props = $props();

  /// What is drawn: the value picked from the menu, the view's first until one is
  /// (`decisions.md#ui`, 2026-10-12 — a screen shows what was picked, and at most scrolls).
  let picked = $state<string | null>(null);
  const primary = $derived(
    def.metrics.find((m) => m.key === picked) ??
      def.metrics.find((m) => m.role === 'primary') ??
      def.metrics[0],
  );
  const others = $derived(def.metrics.filter((m) => m !== primary));
  const TINTS = ['a', 'b', 'c'] as const;
  const measured = (m: Metric) => m.agg === 'mean';

  /// A day's value for a metric: its sum, or — for a measured one — the mean of its entries.
  /// `null` when no entry that day carried it.
  function valueOn(d: DayTotals, m: Metric): number | null {
    const got = d.sums[m.key];
    if (!got || got.n === 0) return null;
    return measured(m) ? got.sum / got.n : got.sum;
  }
  const logged = $derived(days.filter((d) => d.of > 0));
  /// An average over the days that have a value, never over the empty ones.
  function avg(m: Metric): number {
    const vals = days.map((d) => valueOn(d, m)).filter((v): v is number => v !== null);
    return vals.length ? vals.reduce((t, v) => t + v, 0) / vals.length : 0;
  }
  const targetOf = (key: string) => days.at(-1)?.targets[key] ?? 0;
  /// How many of the logged entries carried this value, out of how many there were.
  const carried = (key: string) => ({
    n: logged.reduce((t, d) => t + (d.sums[key]?.n ?? 0), 0),
    of: logged.reduce((t, d) => t + d.of, 0),
  });

  /// For a measured value: where it is now, and how far it moved over the days shown.
  const series = $derived(days.map((d) => valueOn(d, primary)));
  const known = $derived(series.filter((v): v is number => v !== null));
  const latest = $derived(known.at(-1) ?? 0);
  const moved = $derived(known.length > 1 ? latest - known[0] : 0);
  const asLine = $derived(measured(primary) || days.length > 31);

  /// When the entries of these days were, for the days that carry times: how many occasions a
  /// day, and the usual first and last — the middle value, so one late night does not move it.
  /// A count of what happened; nobody gives a number to reach (`sources-2026-10-11.md` §6c).
  const timed = $derived(days.filter((d) => d.times));
  const middle = (list: string[]) => [...list].sort()[Math.floor((list.length - 1) / 2)] ?? '';
  const rhythm = $derived(
    timed.length
      ? {
          occasions:
            Math.round((timed.reduce((t, d) => t + d.times!.occasions, 0) / timed.length) * 10) /
            10,
          first: middle(timed.map((d) => d.times!.first)),
          last: middle(timed.map((d) => d.times!.last)),
        }
      : null,
  );
</script>

<div class="trends">
  <div class="top">
    <div class="seg" role="group" aria-label="how many days">
      {#each spans as s (s.days)}
        <button type="button" class:on={span === s.days} onclick={() => onspan(s.days)}
          >{s.label}</button
        >
      {/each}
    </div>
    {#if def.metrics.length > 1}
      <label class="pick">
        <span class="sr">what to show</span>
        <select value={primary.key} onchange={(e) => (picked = e.currentTarget.value)}>
          {#each def.metrics as m (m.key)}
            <option value={m.key}>{m.label}</option>
          {/each}
        </select>
      </label>
    {/if}
    {#if onadd || (onmeasure && primary.log)}
      <button
        type="button"
        class="add"
        onclick={() => (primary.log && onmeasure ? onmeasure(primary) : onadd?.())}
        aria-label={`add ${primary.label}`}
      >
        <Icon name="plus" size={18} />
      </button>
    {/if}
  </div>

  {#if days.length && !logged.length && !known.length}
    <EmptyState icon="chart" title="Nothing logged in these days" />
  {:else}
    <section class="card">
      {#if measured(primary)}
        <p class="lead">
          <strong>{show(primary.key, Math.round(latest)).text}</strong>
          {show(primary.key, latest).unit}
          {#if moved !== 0}
            <span class="sub"
              >· {moved > 0 ? '+' : '−'}{showWith(primary.key, Math.abs(Math.round(moved)), latest)} over
              these days</span
            >
          {/if}
        </p>
      {:else}
        <p class="lead">
          <strong>{show(primary.key, Math.round(avg(primary))).text}</strong>
          {show(primary.key, avg(primary)).unit} a day
          {#if targetOf(primary.key) > 0}
            <span class="sub">· target {showWith(primary.key, targetOf(primary.key))}</span>
          {/if}
        </p>
      {/if}
      {#if asLine}
        <Line
          points={days.map((d, i) => ({
            key: d.day,
            value: series[i],
            target: d.targets[primary.key],
          }))}
          label={`${primary.label} over ${days.length} days`}
        />
      {:else}
        <Bars
          bars={days.map((d, i) => ({
            key: d.day,
            value: series[i] ?? 0,
            target: d.targets[primary.key] ?? 0,
            label: `${dayLabel(d.day)}: ${showWith(primary.key, Math.round(series[i] ?? 0))}`,
            tick: days.length <= 7 ? dayLetter(d.day) : i % 7 === 0 ? String(dayNumber(d.day)) : '',
          }))}
          onpick={onday}
        />
      {/if}
      <p class="honest">
        Logged {primary.log ? known.length : logged.length} of {days.length} days
      </p>
    </section>

    {#if rhythm}
      <section class="card">
        <div class="line">
          <span class="name">Times you ate a day</span>
          <span class="val">{rhythm.occasions}</span>
        </div>
        <div class="line">
          <span class="name">Usual first and last</span>
          <span class="val">{rhythm.first} – {rhythm.last}</span>
        </div>
        <p class="honest">
          From the {timed.length} of {days.length} days whose entries have a time. Entries less than 15
          minutes apart count as one.
        </p>
      </section>
    {/if}
    {#if others.length}
      <section class="card">
        {#each others as m, i (m.key)}
          {@const a = avg(m)}
          {@const t = targetOf(m.key)}
          {@const c = carried(m.key)}
          <div class="line">
            <span class="name">{m.label}</span>
            <span class="val"
              >{show(m.key, Math.round(a), t || a).text}{#if t > 0}<span class="sub"
                  >&nbsp;/ {showWith(m.key, t)}</span
                >{:else}&nbsp;{show(m.key, a).unit}{/if}</span
            >
            {#if t > 0}
              <Meter
                value={a}
                target={t}
                tint={TINTS[i % 3]}
                label={`${m.label}: ${showWith(m.key, Math.round(a))} a day`}
              />
            {/if}
            {#if !m.log && c.n < c.of * 0.9}
              <span class="partial">from {c.n} of {c.of} entries</span>
            {/if}
          </div>
        {/each}
        {#if others.some((m) => !m.log && carried(m.key).n < carried(m.key).of)}
          <p class="honest">Some entries carry only part of these, so they run a little low.</p>
        {/if}
      </section>
    {/if}
  {/if}
</div>

<style>
  .trends {
    height: 100%;
    overflow-y: auto;
    overscroll-behavior: contain;
    box-sizing: border-box;
    max-width: 34rem;
    margin: 0 auto;
    padding: var(--space-3) var(--space-3) var(--space-8);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .trends > :global(*) {
    flex-shrink: 0;
  }
  .top {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
  }
  .add {
    width: 2.5rem;
    height: 2.5rem;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: var(--radius-pill);
    background: var(--accent);
    color: var(--accent-contrast);
    cursor: pointer;
  }
  /* Where the pane is narrow the group's own bar is on screen, and it has this control already. */
  @container pane (max-width: 60rem) {
    .add {
      display: none;
    }
  }
  .seg {
    display: flex;
    padding: 2px;
    border-radius: var(--radius-pill);
    background: var(--surface-hover);
  }
  .seg button {
    min-height: 2.25rem;
    padding: 0 var(--space-4);
    border: 0;
    border-radius: var(--radius-pill);
    background: none;
    color: var(--text-muted);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }
  .seg button.on {
    background: var(--surface-elevated);
    color: var(--text);
    box-shadow: var(--shadow-sm);
  }
  .card {
    padding: var(--space-4);
    border-radius: var(--radius-lg);
    background: var(--surface-elevated);
    box-shadow: var(--shadow-sm);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .lead {
    margin: 0;
    color: var(--text-muted);
  }
  .lead strong {
    font-size: var(--text-2xl);
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }
  .honest {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .line {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 2px var(--space-2);
    font-size: var(--text-sm);
  }
  .line :global(.meter),
  .partial {
    grid-column: 1 / -1;
  }
  .val {
    font-variant-numeric: tabular-nums;
  }
  .sub {
    color: var(--text-subtle);
    font-size: var(--text-sm);
  }
  .partial {
    font-size: var(--text-xs);
    color: var(--text-subtle);
  }
  .pick select {
    min-height: 2.25rem;
    padding: 0 var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    background: var(--surface);
    color: inherit;
    font: inherit;
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
