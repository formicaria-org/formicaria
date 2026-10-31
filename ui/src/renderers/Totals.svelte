<script lang="ts">
  // One day of a log: what remains of the first metric's target, the others against theirs, a strip
  // of the days around it, and the day's entries under their slots.
  //
  // **Generic, and it stays that way.** Which properties are summed, what they are called and how
  // the day is divided all come from the view's definition; the unit comes from the key
  // (`units.ts`). Nothing here knows what is being logged. What a person *does* about an entry —
  // the sheet that adds one, the step that changes an amount — is not this component's business;
  // it reports the gesture and its owner decides (`decisions.md#ui`, 2026-10-11).
  import { untrack } from 'svelte';
  import { slide } from 'svelte/transition';
  import { Tween } from 'svelte/motion';
  import Ring from '../lib/charts/Ring.svelte';
  import Meter from '../lib/charts/Meter.svelte';
  import Icon from '../lib/Icon.svelte';
  import { dur, tween } from '../lib/charts/motion';
  import { clockOf, show, showWith, whole, dayLabel, dayLetter, dayNumber } from '../lib/units';
  import { dayAgainst, dishesFor, gap, shownAsk, shownValue } from '../lib/plan/measure';
  import type { Dish } from '../lib/plan/measure';
  import ExampleDay from '../lib/plan/ExampleDay.svelte';
  import { shownAmount } from '../lib/food/amount';
  import type {
    DayTotals,
    ObjectMeta,
    PlanCheckProgress,
    PlanFoodExample,
    PlanProgress,
    TotalsDef,
  } from '../lib/types';

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
    /** Take one entry out, on its own — offered beside every entry when its owner can do it. */
    onremove?: (row: ObjectMeta) => void;
    /** The day against what the person follows, when they follow something
     *  (`decisions.md#ui`, 2026-10-12, *the day says whether it follows the diet*). */
    plan?: PlanProgress | null;
    /** Open every vitamin and mineral of the day. Offered only when its owner can show them. */
    onnutrients?: () => void;
    /** Choose the values Day follows. Offered only when its owner can keep the choice. */
    onchoose?: () => void;
    /** What each followed value fills towards when no target was set: the nutrients sheet's
     *  reference for the person, or its limit (`log_days`' `references`). */
    references?: Record<string, { reference?: number; upper?: number }>;
    /** Something was planned from an example day: ask for the day again. */
    onexample?: () => void;
    /** Dishes a serving of which could be had: the person's recipes and the carried ones. */
    dishes?: Dish[];
    /** A dish offered was chosen: open it, to have or to plan. */
    onidea?: (dish: Dish) => void;
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
    onremove,
    plan = null,
    onnutrients,
    onchoose,
    references = {},
    dishes = [],
    onidea,
    onexample,
  }: Props = $props();

  /// The diet's own examples of a day, for the fold in its strip.
  const dayExamples = $derived(
    (plan?.plan?.examples ?? []).filter((e): e is PlanFoodExample => !('week' in e)),
  );

  const against = $derived(plan?.plan ? dayAgainst(plan.checks) : null);
  /// Which check is opened to show what made it up.
  let opened = $state<string | null>(null);
  /// The day's entries that put most into a check's value — for a check of one value.
  function makers(c: PlanCheckProgress): { title: string; text: string }[] {
    const key = plan?.plan?.checks.find((x) => x.id === c.id)?.key;
    if (!key) return [];
    return (day?.rows ?? [])
      .filter((r) => !r.props.planned && (whole(r.props[key]) ?? 0) > 0)
      .sort((a, b) => (whole(b.props[key]) ?? 0) - (whole(a.props[key]) ?? 0))
      .slice(0, 3)
      .map((r) => ({
        title: r.title ?? 'Untitled',
        text: showWith(key, whole(r.props[key]) ?? 0),
      }));
  }

  const primary = $derived(def.metrics.find((m) => m.role === 'primary') ?? def.metrics[0]);
  const secondary = $derived(def.metrics.filter((m) => m.role === 'secondary'));
  const detail = $derived(def.metrics.filter((m) => m.role === 'detail'));
  /// **The values the person follows: three small rings under the day's own** (owner, 2026-10-31,
  /// later: *"up to a few entries (say 3) where a person can select what to track and visualize how
  /// much they are taking it that day"*). Each fills towards the person's own target, else the
  /// nutrients sheet's reference for them. A limit is not something to fill: its ring stays empty
  /// and the limit is written under it.
  const followed = $derived(detail.slice(0, 3));
  const aim = (key: string) => {
    const set = day?.targets[key] ?? 0;
    if (set > 0) return { to: set, limit: 0 };
    const r = references[key];
    return { to: r?.reference ?? 0, limit: r?.reference ? 0 : (r?.upper ?? 0) };
  };
  const TINTS = ['a', 'b', 'c'] as const;

  const sumOf = (d: DayTotals | null, key: string) => d?.sums[key]?.sum ?? 0;
  const eaten = $derived(sumOf(day, primary.key));
  const target = $derived(day?.targets[primary.key] ?? 0);
  /// The energy still under the day's ceiling, when it has one: a dish offered must fit in it.
  const room = $derived(target > 0 ? Math.max(0, target - eaten) : null);
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
  /// **A meal is one line until it is opened** (owner, 2026-10-31, later: *"breakfast, lunch … is
  /// nice but is there a way to have them in the view but take less space?"*): its name, how many
  /// foods, what they come to and its ＋; pressed, its foods. Only where the view divides the day
  /// into slots — a day of training has none and shows its sets as before.
  const folding = $derived(def.slots.length > 0);
  let opened_slots = $state<string[]>([]);
  const isOpen = (v: string) => !folding || opened_slots.includes(v);
  const flip = (v: string) =>
    (opened_slots = opened_slots.includes(v)
      ? opened_slots.filter((x) => x !== v)
      : [...opened_slots, v]);
  /// A meal that gains an entry on the day on screen opens, so what was just added is in sight.
  let seen: { day: string; n: Record<string, number> } | null = null;
  $effect(() => {
    if (!day || day.day !== selected) return;
    const n = Object.fromEntries(slots.map((x) => [x.value, (bySlot.get(x.value) ?? []).length]));
    const was = seen?.day === day.day ? seen.n : null;
    seen = { day: day.day, n };
    if (!was) return;
    const grew = Object.keys(n).filter((k) => n[k] > (was[k] ?? 0));
    if (grew.length) untrack(() => (opened_slots = [...new Set([...opened_slots, ...grew])]));
  });
  const slotSum = (list: ObjectMeta[]) =>
    list.reduce((t, r) => t + (r.props.planned ? 0 : (whole(r.props[primary.key]) ?? 0)), 0);
  /// An entry's amount — and, when it is only known roughly, the least and the most it could be.
  const amountOf = (r: ObjectMeta) => {
    const k = Object.keys(r.props).find((p) => /^amount_(?!min_|max_)/.test(p));
    const v = k ? whole(r.props[k]) : null;
    if (!k || v === null) return '';
    const unit = k.slice('amount_'.length);
    // A count of pieces says what they are: "2 capsules" (2026-10-31).
    if (unit === 'each') return shownAmount(r.props);
    const min = whole(r.props[`amount_min_${unit}`]);
    const max = whole(r.props[`amount_max_${unit}`]);
    return min !== null && max !== null && min < max
      ? `${show(k, min).text}–${showWith(k, max)}`
      : showWith(k, v);
  };
  /// The least and the most the day's first value could be, when an amount was a range.
  const between = $derived.by(() => {
    const got = day?.sums[primary.key];
    return got?.low !== undefined && got?.high !== undefined && got.low < got.high
      ? `between ${show(primary.key, got.low).text} and ${showWith(primary.key, got.high)}`
      : '';
  });

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
        {#if between}<span class="between">{between}</span>{/if}
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

  {#if followed.length || onchoose}
    <section class="follow" aria-label="what you follow">
      <ul>
        {#each followed as m (m.key)}
          {@const got = day?.sums[m.key]}
          {@const v = got?.sum ?? 0}
          {@const a = aim(m.key)}
          {@const partial = !!day && day.of > 0 && (got?.n ?? 0) < day.of}
          <li>
            <Ring
              value={v}
              target={a.to}
              size={64}
              stroke={7}
              label={`${m.label}: ${showWith(m.key, v, a.to || v)}${a.to ? ` of ${showWith(m.key, a.to)}` : ''}`}
            >
              <span class="mini">{showWith(m.key, v, a.to || a.limit || v)}</span>
            </Ring>
            <span class="name">{m.label}</span>
            <span class="of"
              >{#if a.to}of {showWith(m.key, a.to)}{:else if a.limit}{v > a.limit
                  ? 'over the limit of'
                  : 'limit'}
                {showWith(m.key, a.limit)}{/if}</span
            >
            {#if partial}<span class="partial">from {got?.n ?? 0} of {day?.of}</span>{/if}
          </li>
        {/each}
      </ul>
      {#if onchoose || onnutrients}
        <div class="follow-actions">
          {#if onchoose}
            <button type="button" class="more" onclick={onchoose}
              >{followed.length ? 'Choose' : 'Choose what to follow'}</button
            >
          {/if}
          {#if onnutrients}
            <button type="button" class="more" onclick={onnutrients}>All nutrients</button>
          {/if}
        </div>
      {/if}
    </section>
  {/if}

  {#if against && plan?.plan}
    <section class="diet" aria-label="today against what you follow">
      <header>
        <h3>{plan.plan.name}</h3>
        <span class="tally">
          {#if against.nothing}
            nothing logged yet
          {:else if against.daily}
            {against.met} of {against.daily} met today
          {/if}
        </span>
      </header>
      {#if against.daily && !against.nothing && !against.off.length}
        <p class="fine">Today is within everything it asks of a day.</p>
      {/if}
      {#if against.off.length}
        <ul>
          {#each against.off as c (c.id)}
            {@const from = makers(c)}
            <li>
              <button
                type="button"
                class="off"
                aria-expanded={opened === c.id}
                onclick={() => (opened = opened === c.id ? null : c.id)}
              >
                <span class="what">{c.label}</span>
                <span class="now"
                  >{c.value !== undefined && c.value !== null ? shownValue(c, c.value) : ''}
                  <span class="sub">· {shownAsk(c)}</span></span
                >
                <span class="gap" class:over={c.state === 'over'}>{gap(c)}</span>
              </button>
              {#if opened === c.id}
                <div class="why">
                  {#if from.length}
                    <p>Most of it today came from:</p>
                    <ul>
                      {#each from as f (f.title + f.text)}
                        <li><span>{f.title}</span><span>{f.text}</span></li>
                      {/each}
                    </ul>
                  {:else}
                    <p>It is worked out from the whole day, not from one value of each entry.</p>
                  {/if}
                  {#each [dishesFor(c, dishes, room, 3, plan?.checks ?? [])] as ideas (c.id)}
                    {#if ideas.length}
                      <p>
                        A serving of these has most of it{room !== null
                          ? ', and fits what is left of your energy'
                          : ''}. It would bring you to:
                      </p>
                      <ul>
                        {#each ideas as x (x.dish.from + x.dish.title)}
                          <li>
                            <button
                              type="button"
                              class="idea"
                              onclick={() => onidea?.(x.dish)}
                              aria-label={`open ${x.dish.title}`}
                            >
                              <span
                                >{x.dish.title}
                                <span class="sub"
                                  >· {x.dish.from}{x.dish.have
                                    ? ` · you have ${x.dish.have.n} of its ${x.dish.have.of} ingredients`
                                    : ''}</span
                                ></span
                              >
                              <span class="to">{shownValue(c, x.reaches)}</span>
                            </button>
                            {#if x.over.length}
                              <p class="but">
                                but it would take {x.over.map((l) => l.toLowerCase()).join(' and ')} past
                                {x.over.length === 1 ? 'its' : 'their'} limit
                              </p>
                            {/if}
                          </li>
                        {/each}
                      </ul>
                    {/if}
                  {/each}
                  <p class="basis">{c.basis}</p>
                </div>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
      {#if dayExamples.length && onexample}
        <details class="start">
          <summary>A day to start from</summary>
          {#each dayExamples as example, i (i)}
            <ExampleDay {example} {today} onsaved={onexample} />
          {/each}
        </details>
      {/if}
      {#each against.weekly as c (c.id)}
        <p class="weekly">
          {c.label}: {c.value !== undefined && c.value !== null
            ? shownValue(c, c.value)
            : 'nothing yet'}
          <span class="sub">· {shownAsk(c)}</span>
        </p>
      {/each}
    </section>
  {/if}
  {#if day?.times}
    <p class="times">
      {day.times.occasions === 1 ? 'Once' : `${day.times.occasions} times`}
      {#if day.times.first === day.times.last}
        · at {day.times.first}
      {:else}
        · first at {day.times.first} · last at {day.times.last}
      {/if}
    </p>
  {/if}
  {#each slots as s (s.value)}
    {@const list = bySlot.get(s.value) ?? []}
    {@const copyable = !list.length && !!oncopy && (yesterday[s.value] ?? 0) > 0}
    {@const foldable = folding && (list.length > 0 || copyable)}
    <section class="slot" class:folded={folding && !isOpen(s.value)}>
      <header>
        {#if foldable}
          <button
            type="button"
            class="fold"
            aria-expanded={isOpen(s.value)}
            onclick={() => flip(s.value)}
          >
            <h3>{s.label}</h3>
            {#if list.length}<span class="count">{list.length} ·</span>{/if}
            <span class="sum">{list.length ? show(primary.key, slotSum(list)).text : '—'}</span>
            <span class="chev" class:down={isOpen(s.value)}
              ><Icon name="chevronRight" size={16} /></span
            >
          </button>
        {:else}
          <h3>{s.label}</h3>
          <span class="sum">{list.length ? show(primary.key, slotSum(list)).text : '—'}</span>
        {/if}
        <button
          type="button"
          class="add"
          onclick={() => onadd(s.value)}
          aria-label={`add to ${s.label}`}
        >
          <Icon name="plus" size={18} />
        </button>
      </header>
      {#if !isOpen(s.value)}
        <!-- Folded: the line above is the meal. -->
      {:else if list.length}
        <ul transition:slide={{ duration: dur(180) }}>
          {#each list as r (r.id)}
            <li in:slide={{ duration: dur(220) }} class:planned={!!r.props.planned}>
              <button type="button" class="entry" onclick={() => onedit(r)}>
                <span class="title"
                  >{r.title ?? 'Untitled'}{#if clockOf(r.start)}<span class="at"
                      >{clockOf(r.start)}</span
                    >{/if}</span
                >
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
              <!-- **One entry out, on its own, in plain sight** (owner, 2026-10-18: two coffees
                   added by mistake, and to fix it the whole breakfast was removed). Changing the
                   amount was always a tap on the row; nothing said so. Each step can be undone. -->
              {#if onremove}
                <button
                  type="button"
                  class="drop"
                  onclick={() => onremove(r)}
                  aria-label={`remove ${r.title ?? 'entry'}`}
                  title="Remove this one"
                >
                  <Icon name="close" size={16} />
                </button>
              {/if}
            </li>
          {/each}
        </ul>
      {:else if copyable && oncopy}
        <button type="button" class="again" onclick={() => oncopy(s.value)}>
          Same as yesterday ({yesterday[s.value]})
        </button>
      {/if}
    </section>
  {/each}
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
  .follow {
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
  /* A meal's line, pressed to open it: the whole line but its ＋. */
  .fold {
    flex: 1;
    min-width: 0;
    min-height: 2.75rem;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 0;
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .count {
    color: var(--text-subtle);
    font-size: var(--text-sm);
    font-variant-numeric: tabular-nums;
  }
  .chev {
    display: grid;
    color: var(--text-subtle);
    transition: transform var(--dur-fast, 120ms) ease;
  }
  .chev.down {
    transform: rotate(90deg);
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
  .drop {
    flex: none;
    width: 2.5rem;
    height: 2.5rem;
    margin: var(--space-1) var(--space-2) var(--space-1) 0;
    display: grid;
    place-items: center;
    border: none;
    border-radius: var(--radius-pill);
    background: none;
    color: var(--text-muted);
    cursor: pointer;
  }
  .drop:hover {
    background: var(--surface-hover);
    color: var(--text);
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

  /* The values the person follows: three small rings side by side. */
  .follow {
    padding: var(--space-3);
  }
  .follow ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--space-2);
  }
  .follow li {
    border-top: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    min-width: 0;
    text-align: center;
  }
  .mini {
    font-size: var(--text-xs);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .follow .name {
    font-size: var(--text-sm);
  }
  .follow .of {
    font-size: var(--text-xs);
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }
  .follow-actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
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
  .times {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
    text-align: center;
  }
  .at {
    margin-left: var(--space-2);
    font-size: var(--text-xs);
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }
  .start summary {
    min-height: 2.5rem;
    cursor: pointer;
    font-size: var(--text-sm);
  }
  .diet {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }
  .diet header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-2);
  }
  .diet h3 {
    margin: 0;
    font-size: var(--text-md);
  }
  .diet .tally,
  .diet .fine,
  .diet .weekly,
  .diet .basis {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .diet ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  .diet > ul > li {
    display: block;
    padding: 0;
    border: 0;
  }
  .why ul li {
    display: flex;
    flex-wrap: wrap;
    padding: 0;
    border: 0;
  }
  .off {
    width: 100%;
    box-sizing: border-box;
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 0 var(--space-2);
    align-items: baseline;
    min-height: 2.75rem;
    padding: var(--space-2);
    border: 0;
    border-radius: var(--radius-md);
    background: var(--surface-hover);
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .off .what {
    font-weight: 600;
  }
  .off .now {
    grid-column: 1;
    font-size: var(--text-sm);
  }
  .off .gap {
    grid-row: 1 / span 2;
    grid-column: 2;
    align-self: center;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .off .gap.over {
    color: var(--danger-fg);
  }
  .why {
    padding: var(--space-2);
    font-size: var(--text-sm);
  }
  .why p {
    margin: 0 0 var(--space-1);
  }
  .idea {
    width: 100%;
    box-sizing: border-box;
    min-height: 2.75rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: none;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .why .but {
    margin: 0 0 var(--space-1) var(--space-2);
    font-size: var(--text-xs);
    color: var(--danger-fg);
  }
  .why .to {
    flex: none;
    font-variant-numeric: tabular-nums;
  }
  .why li {
    display: flex;
    justify-content: space-between;
    gap: var(--space-2);
  }
  .more {
    align-self: flex-start;
    min-height: 2.75rem;
    margin-top: var(--space-2);
    padding: 0 var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  .between {
    display: block;
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
</style>
