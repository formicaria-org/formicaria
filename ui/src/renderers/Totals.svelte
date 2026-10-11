<script lang="ts">
  // One day of a log: what remains of the first metric's target, the others against theirs, a strip
  // of the days around it, and the day's entries under their slots.
  //
  // **Generic, and it stays that way.** Which properties are summed, what they are called and how
  // the day is divided all come from the view's definition; the unit comes from the key
  // (`units.ts`). Nothing here knows what is being logged. What a person *does* about an entry —
  // the sheet that adds one, the step that changes an amount — is not this component's business;
  // it reports the gesture and its owner decides (`decisions.md#ui`, 2026-10-11).
  import { slide } from 'svelte/transition';
  import { Tween } from 'svelte/motion';
  import Ring from '../lib/charts/Ring.svelte';
  import Meter from '../lib/charts/Meter.svelte';
  import Icon from '../lib/Icon.svelte';
  import { dur, tween } from '../lib/charts/motion';
  import { show, showWith, whole, dayLabel, dayLetter, dayNumber } from '../lib/units';
  import type { DayTotals, ObjectMeta, TotalsDef } from '../lib/types';

  interface Props {
    def: TotalsDef;
    /** The chosen day, with its entries — `null` while it is on its way. */
    day: DayTotals | null;
    /** The days offered in the strip, oldest first. */
    strip: DayTotals[];
    selected: string;
    today: string;
    /** How many entries the same slot held the day before, for the "same as yesterday" offer. */
    yesterday?: Record<string, number>;
    onday: (day: string) => void;
    onadd: (slot: string | null) => void;
    onedit: (row: ObjectMeta) => void;
    oncopy?: (slot: string) => void;
    /** Change what the day is measured against. Offered only when its owner can do it. */
    ontargets?: () => void;
    /** A planned entry happened: count it. Offered only when its owner can do it. */
    onconfirm?: (row: ObjectMeta) => void;
  }
  let {
    def,
    day,
    strip,
    selected,
    today,
    yesterday = {},
    onday,
    onadd,
    onedit,
    oncopy,
    ontargets,
    onconfirm,
  }: Props = $props();

  const primary = $derived(def.metrics.find((m) => m.role === 'primary') ?? def.metrics[0]);
  const secondary = $derived(def.metrics.filter((m) => m.role === 'secondary'));
  const detail = $derived(def.metrics.filter((m) => m.role === 'detail'));
  const TINTS = ['a', 'b', 'c'] as const;

  const sumOf = (d: DayTotals | null, key: string) => d?.sums[key]?.sum ?? 0;
  const eaten = $derived(sumOf(day, primary.key));
  const target = $derived(day?.targets[primary.key] ?? 0);
  /// What the middle of the ring says: what remains while there is a target and room under it, how
  /// far past it once there is not, and the plain total when there is no target at all.
  const hero = $derived(
    target <= 0
      ? { n: eaten, word: show(primary.key, eaten).unit }
      : eaten <= target
        ? { n: target - eaten, word: 'left' }
        : { n: eaten - target, word: 'over' },
  );
  const counted = new Tween(0, tween());
  $effect(() => {
    counted.target = hero.n;
  });

  const rows = $derived(day?.rows ?? []);
  const bySlot = $derived.by(() => {
    const m = new Map<string, ObjectMeta[]>();
    for (const r of rows) {
      const k = String(r.props.slot ?? '');
      m.set(k, [...(m.get(k) ?? []), r]);
    }
    return m;
  });
  /// The slots drawn: the definition's, then any an entry names that the definition does not, so
  /// nothing logged is ever missing from its own day.
  const slots = $derived([
    ...def.slots,
    ...[...bySlot.keys()]
      .filter((k) => !def.slots.some((s) => s.value === k))
      .map((k) => ({ value: k, label: k || 'Other' })),
  ]);
  const slotSum = (list: ObjectMeta[]) =>
    list.reduce((t, r) => t + (r.props.planned ? 0 : (whole(r.props[primary.key]) ?? 0)), 0);
  const amountOf = (r: ObjectMeta) => {
    const k = Object.keys(r.props).find((p) => p.startsWith('amount_'));
    const v = k ? whole(r.props[k]) : null;
    return k && v !== null ? showWith(k, v) : '';
  };

  let stripEl = $state<HTMLElement | null>(null);
  $effect(() => {
    // Keep the chosen day in sight when it changes.
    void selected;
    void strip.length;
    stripEl?.querySelector('[aria-current="date"]')?.scrollIntoView?.({
      inline: 'center',
      block: 'nearest',
    });
  });
  const step = (n: number) => {
    const at = strip.findIndex((d) => d.day === selected);
    const next = strip[at + n];
    if (next) onday(next.day);
  };
</script>

<div class="totals">
  <header class="when">
    <button type="button" class="nudge" onclick={() => step(-1)} aria-label="the day before">
      <Icon name="chevronLeft" size={18} />
    </button>
    <button
      type="button"
      class="date"
      onclick={() => onday(today)}
      title="Back to today"
      aria-label={selected === today ? `today, ${dayLabel(selected)}` : dayLabel(selected)}
    >
      {#if selected === today}<strong>Today</strong>{:else if selected > today}<strong
          >Planned</strong
        >{/if}
      <span>{dayLabel(selected)}</span>
    </button>
    <button
      type="button"
      class="nudge"
      onclick={() => step(1)}
      disabled={strip.findIndex((d) => d.day === selected) >= strip.length - 1}
      aria-label="the day after"
    >
      <Icon name="chevronRight" size={18} />
    </button>
  </header>

  <div class="strip" bind:this={stripEl}>
    {#each strip as d (d.day)}
      {@const t = d.targets[primary.key] ?? 0}
      {@const v = sumOf(d, primary.key)}
      <button
        type="button"
        class="pip"
        class:on={d.day === selected}
        aria-current={d.day === selected ? 'date' : undefined}
        aria-label={`${dayLabel(d.day)}: ${showWith(primary.key, v)}`}
        onclick={() => onday(d.day)}
      >
        <span class="letter">{dayLetter(d.day)}</span>
        <Ring value={v} target={t} size={30} stroke={4} label="">
          <span class="num">{dayNumber(d.day)}</span>
        </Ring>
      </button>
    {/each}
  </div>

  <section class="hero" class:waiting={!day}>
    <Ring
      value={eaten}
      {target}
      label={target > 0
        ? `${showWith(primary.key, eaten)} of ${showWith(primary.key, target)}`
        : showWith(primary.key, eaten)}
    >
      <div class="count">
        <span class="big">{show(primary.key, Math.round(counted.current)).text}</span>
        <span class="word">{hero.word}</span>
      </div>
    </Ring>
    <div class="side">
      <p class="of">
        {#if target > 0}
          <strong>{show(primary.key, eaten).text}</strong> of {showWith(primary.key, target)}
        {:else}
          {primary.label}
        {/if}
        {#if ontargets}
          <button type="button" class="targets" onclick={ontargets}>
            {target > 0 ? 'Change' : 'Set a target'}
          </button>
        {/if}
      </p>
      {#each secondary as m, i (m.key)}
        {@const v = sumOf(day, m.key)}
        {@const t = day?.targets[m.key] ?? 0}
        <div class="line">
          <span class="name">{m.label}</span>
          <span class="val"
            >{show(m.key, v, t || v).text}{#if t > 0}<span class="sub"
                >&nbsp;/ {showWith(m.key, t)}</span
              >{:else}&nbsp;{show(m.key, v).unit}{/if}</span
          >
          <Meter
            value={v}
            target={t}
            tint={TINTS[i % 3]}
            label={`${m.label}: ${showWith(m.key, v)}`}
          />
        </div>
      {/each}
    </div>
  </section>

  {#each slots as s (s.value)}
    {@const list = bySlot.get(s.value) ?? []}
    <section class="slot">
      <header>
        <h3>{s.label}</h3>
        <span class="sum">{list.length ? show(primary.key, slotSum(list)).text : '—'}</span>
        <button
          type="button"
          class="add"
          onclick={() => onadd(s.value)}
          aria-label={`add to ${s.label}`}
        >
          <Icon name="plus" size={18} />
        </button>
      </header>
      {#if list.length}
        <ul>
          {#each list as r (r.id)}
            <li in:slide={{ duration: dur(220) }} class:planned={!!r.props.planned}>
              <button type="button" class="entry" onclick={() => onedit(r)}>
                <span class="title">{r.title ?? 'Untitled'}</span>
                <span class="amount">{amountOf(r)}</span>
                <span class="value">{show(primary.key, whole(r.props[primary.key]) ?? 0).text}</span
                >
              </button>
              <!-- Planned, not yet had: it is in no total until it is confirmed. -->
              {#if r.props.planned && onconfirm}
                <button
                  type="button"
                  class="confirm"
                  onclick={() => onconfirm(r)}
                  aria-label={`confirm ${r.title ?? 'entry'}`}
                >
                  <Icon name="check" size={18} />
                </button>
              {/if}
            </li>
          {/each}
        </ul>
      {:else if oncopy && (yesterday[s.value] ?? 0) > 0}
        <button type="button" class="again" onclick={() => oncopy(s.value)}>
          Same as yesterday ({yesterday[s.value]})
        </button>
      {/if}
    </section>
  {/each}

  {#if day && detail.length && day.of > 0}
    <section class="detail">
      <h3>Also today</h3>
      <dl>
        {#each detail as m (m.key)}
          {@const got = day.sums[m.key]}
          {@const t = day.targets[m.key] ?? 0}
          <div>
            <dt>{m.label}</dt>
            <dd>
              {showWith(m.key, got?.sum ?? 0, t || (got?.sum ?? 0))}{#if t > 0}<span class="sub"
                  >&nbsp;/ {showWith(m.key, t)}</span
                >{/if}
              {#if (got?.n ?? 0) < day.of}
                <span class="partial" title="Some entries have no value for this"
                  >from {got?.n ?? 0} of {day.of}</span
                >
              {/if}
            </dd>
          </div>
        {/each}
      </dl>
    </section>
  {/if}
</div>

<style>
  .totals {
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
  button {
    font: inherit;
    color: inherit;
  }
  .when {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }
  .date {
    flex: 1;
    min-height: 2.75rem;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.4em;
    border: 0;
    background: none;
    font-size: var(--text-md);
    cursor: pointer;
  }
  .date span {
    color: var(--text-muted);
  }
  /* A column that scrolls: its children keep their own height and the column scrolls past them.
     Left to shrink, every card is squeezed to fit and clips its own rows. */
  .totals > :global(*) {
    flex-shrink: 0;
  }
  .nudge {
    width: 2.75rem;
    height: 2.75rem;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: var(--radius-pill);
    background: none;
    color: var(--text-muted);
    cursor: pointer;
  }
  .nudge:disabled {
    opacity: 0.3;
    cursor: default;
  }
  .nudge:hover:not(:disabled) {
    background: var(--surface-hover);
  }

  .strip {
    display: flex;
    gap: var(--space-1);
    overflow-x: auto;
    scroll-snap-type: x proximity;
    scrollbar-width: none;
    padding-bottom: var(--space-1);
  }
  .strip::-webkit-scrollbar {
    display: none;
  }
  .pip {
    flex: 0 0 calc((100% - 6 * var(--space-1)) / 7);
    scroll-snap-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding: var(--space-1) 0;
    border: 0;
    border-radius: var(--radius-md);
    background: none;
    color: var(--text-muted);
    cursor: pointer;
  }
  .pip.on {
    background: var(--surface-elevated);
    color: var(--text);
    box-shadow: var(--shadow-sm);
  }
  .letter {
    font-size: var(--text-xs);
  }
  .num {
    font-size: 0.625rem;
    font-variant-numeric: tabular-nums;
  }

  .hero {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-4);
    border-radius: var(--radius-lg);
    background: var(--surface-elevated);
    box-shadow: var(--shadow-sm);
    transition: opacity var(--dur-med) var(--ease);
  }
  .hero.waiting {
    opacity: 0.55;
  }
  .count {
    display: flex;
    flex-direction: column;
    line-height: 1.1;
  }
  .big {
    font-size: 2.25rem;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.02em;
  }
  .word {
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .side {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .of {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .targets {
    margin-left: var(--space-1);
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent);
    font-size: var(--text-sm);
    cursor: pointer;
  }
  .of strong {
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }
  .line {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 2px var(--space-2);
    font-size: var(--text-sm);
  }
  .line :global(.meter) {
    grid-column: 1 / -1;
  }
  .val {
    font-variant-numeric: tabular-nums;
  }
  .sub {
    color: var(--text-subtle);
  }

  .slot,
  .detail {
    border-radius: var(--radius-lg);
    background: var(--surface);
    border: 1px solid var(--border);
    overflow: hidden;
  }
  .slot header {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-1) var(--space-1) var(--space-3);
  }
  h3 {
    flex: 1;
    margin: 0;
    font-size: var(--text-md);
    font-weight: 600;
  }
  .sum {
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }
  .add {
    width: 2.75rem;
    height: 2.75rem;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: var(--radius-pill);
    background: none;
    color: var(--accent);
    cursor: pointer;
  }
  .add:hover {
    background: var(--accent-subtle);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    display: flex;
    align-items: center;
    border-top: 1px solid var(--border);
  }
  li.planned .entry {
    color: var(--text-subtle);
    font-style: italic;
  }
  .confirm {
    flex: none;
    width: 2.5rem;
    height: 2.5rem;
    /* Clear of the card's rounded corner, which clips whatever reaches it. */
    margin: var(--space-1) var(--space-2) var(--space-1) 0;
    display: grid;
    place-items: center;
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius-pill);
    background: none;
    color: var(--ok-fg);
    cursor: pointer;
  }
  .entry {
    flex: 1;
    min-width: 0;
    min-height: 2.75rem;
    display: grid;
    grid-template-columns: 1fr auto 3.5rem;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border: 0;
    background: none;
    text-align: left;
    cursor: pointer;
  }
  .entry:hover {
    background: var(--surface-hover);
  }
  .title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .amount {
    color: var(--text-subtle);
    font-size: var(--text-sm);
  }
  .value {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .again {
    width: 100%;
    min-height: 2.75rem;
    padding: var(--space-2) var(--space-3);
    border: 0;
    border-top: 1px solid var(--border);
    background: none;
    color: var(--accent);
    text-align: left;
    cursor: pointer;
  }
  .again:hover {
    background: var(--accent-subtle);
  }

  .detail {
    padding: var(--space-3);
  }
  .detail h3 {
    margin-bottom: var(--space-2);
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  dl {
    margin: 0;
    display: grid;
    gap: var(--space-1);
  }
  dl div {
    display: flex;
    justify-content: space-between;
    gap: var(--space-3);
    font-size: var(--text-sm);
  }
  dt {
    color: var(--text-muted);
  }
  dd {
    margin: 0;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .partial {
    display: block;
    font-size: var(--text-xs);
    color: var(--text-subtle);
  }

  @container pane (max-width: 26rem) {
    .hero {
      gap: var(--space-3);
      padding: var(--space-3);
    }
  }
</style>
