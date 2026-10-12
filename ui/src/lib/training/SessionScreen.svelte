<script lang="ts">
  // A training session as it happens: each exercise, each set, what was lifted last time, and a
  // tick when a set is finished.
  //
  // **A sketch, to settle what a set is before any of it is built for real**
  // (`decisions.md#data`, 2026-10-11): one note per set, planned until it is ticked. Ticking a set
  // clears `planned` and writes `work_g`, the one product a sum cannot compute. The names of the
  // exercises are data and are never written in this file.
  import Icon from '../Icon.svelte';
  import { show, showWith, whole } from '../units';
  import type { DayTotals, ObjectMeta, RecentItem, Routine, TotalsDef } from '../types';

  interface Props {
    def: TotalsDef;
    day: DayTotals | null;
    /** What each exercise was last time, by its name. */
    last: RecentItem[];
    onfinish: (set: ObjectMeta, reps: number, load: number) => void;
    onadd: () => void;
    /** Change a set's numbers, or take it away. */
    onedit: (set: ObjectMeta) => void;
    /** Plan today from the last session there was. */
    onrepeat: () => void;
    /** Sessions kept under a name, and the weekdays each is meant for. */
    routines?: Routine[];
    /** `YYYY-MM-DD`, to know which routines are meant for today. */
    today?: string;
    /** Plan a routine's sets onto today. */
    onplan?: (r: Routine) => void;
    /** Keep today's sets as a routine. */
    onkeep?: () => void;
    /** Change a routine's name or days, or remove it. */
    onroutine?: (r: Routine) => void;
    /** The day shown, in words, when it is not today. */
    label?: string;
    /** Go back to today's session. Offered only when another day is shown. */
    ontoday?: () => void;
  }
  let {
    def,
    day,
    last,
    onfinish,
    onadd,
    onedit,
    onrepeat,
    routines = [],
    today = '',
    onplan,
    onkeep,
    onroutine,
    label = '',
    ontoday,
  }: Props = $props();

  const WEEK = ['sun', 'mon', 'tue', 'wed', 'thu', 'fri', 'sat'];
  const weekday = $derived(today ? WEEK[new Date(`${today}T12:00:00`).getDay()] : '');
  const meant = $derived(routines.filter((r) => r.weekdays.includes(weekday)));
  const rest = $derived(routines.filter((r) => !r.weekdays.includes(weekday)));
  const daysOf = (r: Routine) =>
    r.weekdays.map((d) => d.charAt(0).toUpperCase() + d.slice(1)).join(', ') || 'any day';

  const primary = $derived(def.metrics.find((m) => m.role === 'primary') ?? def.metrics[0]);
  /// A session holds two kinds of entry: a set of an exercise, ticked when done, and an activity
  /// logged by its minutes and effort (`decisions.md#data`, 2026-10-12), which is done when it is
  /// written.
  const activities = $derived((day?.rows ?? []).filter((r) => r.props.activity !== undefined));
  const sets = $derived((day?.rows ?? []).filter((r) => r.props.activity === undefined));
  const groups = $derived.by(() => {
    const m = new Map<string, ObjectMeta[]>();
    for (const s of sets) {
      const k = String(s.props.exercise ?? s.title ?? '');
      m.set(k, [...(m.get(k) ?? []), s]);
    }
    return [...m.entries()].map(([name, list]) => ({
      name,
      list: list.sort((a, b) => (whole(a.props.set) ?? 0) - (whole(b.props.set) ?? 0)),
    }));
  });
  const before = (name: string) => {
    const r = last.find((x) => x.title === name);
    const reps = r ? whole(r.props.reps) : null;
    const load = r ? whole(r.props.load_g) : null;
    return reps !== null && load !== null ? `last ${showWith('load_g', load)} × ${reps}` : '';
  };
  const finished = $derived(sets.filter((s) => !s.props.planned).length);

  // Time since the last set, counted up. **It prescribes nothing**: how long to rest is advice,
  // and advice needs a source (`decisions.md#data`, 2026-10-11, *nothing is invented*). A count
  // of what has passed is only a clock.
  let since = $state<number | null>(null);
  let clock: ReturnType<typeof setInterval> | null = null;
  function startClock() {
    since = 0;
    if (clock) clearInterval(clock);
    clock = setInterval(() => {
      if (since !== null) since += 1;
    }, 1000);
  }
  function stopClock() {
    if (clock) clearInterval(clock);
    clock = null;
    since = null;
  }
  $effect(() => () => {
    if (clock) clearInterval(clock);
  });

  function tick(s: ObjectMeta) {
    if (!s.props.planned) return;
    onfinish(s, whole(s.props.reps) ?? 0, whole(s.props.load_g) ?? 0);
    startClock();
  }
</script>

<div class="session">
  <header class="head">
    <h2>{label ? `Session · ${label}` : 'Today’s session'}</h2>
    {#if ontoday}
      <button type="button" class="quiet" onclick={ontoday}>Back to today</button>
    {/if}
    <p>
      {finished} of {sets.length} sets
      {#if day}· {showWith(primary.key, day.sums[primary.key]?.sum ?? 0, 1000)}{/if}
      {#if activities.length}
        · {activities.reduce((t, a) => t + (whole(a.props.minutes) ?? 0), 0)} min of activity
      {/if}
    </p>
  </header>

  {#if day && sets.length === 0 && activities.length === 0}
    <p class="none">
      {meant.length
        ? `Nothing planned yet. Meant for ${label ? 'this day' : 'today'}:`
        : `Nothing planned for ${label ? 'this day' : 'today'}.`}
    </p>
    <div class="start">
      {#each meant as r (r.id)}
        <button type="button" class="go" onclick={() => onplan?.(r)}>Plan {r.title}</button>
      {/each}
      {#each rest as r (r.id)}
        <button type="button" class="quiet" onclick={() => onplan?.(r)}>Plan {r.title}</button>
      {/each}
      {#if last.length}
        <button type="button" class="go" onclick={onrepeat}>Same as last time</button>
      {/if}
      <button type="button" class="quiet" onclick={onadd}>Add an exercise or an activity</button>
    </div>
  {/if}

  {#if activities.length}
    <section class="exercise">
      <h3>Activities</h3>
      <ul>
        {#each activities as a (a.id)}
          <li class="finished">
            <button
              type="button"
              class="what wide-what"
              onclick={() => onedit(a)}
              aria-label={`change ${a.title ?? 'an activity'}`}
            >
              {a.title ?? a.props.activity} · {whole(a.props.minutes) ?? 0} min · effort {whole(
                a.props.effort,
              ) ?? 0}
            </button>
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  {#each groups as g (g.name)}
    <section class="exercise">
      <h3>{g.name}</h3>
      <ul>
        {#each g.list as s (s.id)}
          {@const open = !!s.props.planned}
          <li class:finished={!open}>
            <span class="n">{whole(s.props.set) ?? ''}</span>
            <span class="ghost">{before(g.name)}</span>
            <button
              type="button"
              class="what"
              onclick={() => onedit(s)}
              aria-label={`change set ${whole(s.props.set) ?? ''} of ${g.name}`}
              >{showWith('load_g', whole(s.props.load_g) ?? 0)} × {whole(s.props.reps) ?? 0}</button
            >
            <button
              type="button"
              class="tick"
              onclick={() => tick(s)}
              aria-pressed={!open}
              aria-label={`set ${whole(s.props.set) ?? ''} of ${g.name}`}
            >
              {#if !open}<Icon name="check" size={18} />{/if}
            </button>
          </li>
        {/each}
      </ul>
    </section>
  {/each}

  {#if sets.length || activities.length}
    <button type="button" class="quiet wide" onclick={onadd}>Add an exercise or an activity</button>
  {/if}
  {#if sets.length && onkeep}
    <button type="button" class="quiet wide" onclick={onkeep}>Keep this session as a routine</button
    >
  {/if}

  {#if routines.length}
    <section class="exercise">
      <h3>Routines</h3>
      <ul>
        {#each routines as r (r.id)}
          <li class="finished">
            <button
              type="button"
              class="what wide-what"
              onclick={() => onroutine?.(r)}
              aria-label={`change the routine ${r.title}`}
            >
              {r.title} · {daysOf(r)} · {r.sets.length}
              {r.sets.length === 1 ? 'set' : 'sets'}
            </button>
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  {#if since !== null}
    <div class="rest" role="timer" aria-label="since the last set">
      <span>Since the last set</span>
      <strong>{show('duration_s', since).text}</strong>
      <button type="button" onclick={stopClock}>Clear</button>
    </div>
  {/if}
</div>

<style>
  .session {
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
  .session > :global(*) {
    flex-shrink: 0;
  }
  .head h2 {
    margin: 0;
    font-size: var(--text-xl);
  }
  .head p,
  .none {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--text-sm);
    font-variant-numeric: tabular-nums;
  }
  .exercise {
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    overflow: hidden;
  }
  h3 {
    margin: 0;
    padding: var(--space-3);
    font-size: var(--text-md);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    display: grid;
    grid-template-columns: 1.5rem 1fr auto 2.75rem;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-2) var(--space-1) var(--space-3);
    border-top: 1px solid var(--border);
    font-variant-numeric: tabular-nums;
    transition: background var(--dur-med) var(--ease);
  }
  li.finished {
    background: var(--ok-bg);
  }
  .n {
    color: var(--text-subtle);
  }
  .ghost {
    color: var(--text-subtle);
    font-size: var(--text-sm);
  }
  .what {
    min-height: 2.75rem;
    padding: 0 var(--space-2);
    border: 0;
    border-radius: var(--radius-md);
    background: none;
    font: inherit;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    color: inherit;
    cursor: pointer;
  }
  .what:hover {
    background: var(--surface-hover);
  }
  .start {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  .go,
  .quiet {
    min-height: 3rem;
    padding: 0 var(--space-4);
    border-radius: var(--radius-md);
    font: inherit;
    cursor: pointer;
  }
  .go {
    border: 0;
    background: var(--accent);
    color: var(--accent-contrast);
    font-weight: 600;
  }
  .quiet {
    border: 1px solid var(--border);
    background: none;
    color: inherit;
  }
  .quiet.wide {
    align-self: stretch;
  }
  .tick {
    width: 2.75rem;
    height: 2.75rem;
    display: grid;
    place-items: center;
    border: 2px solid var(--border-strong);
    border-radius: var(--radius-pill);
    background: none;
    color: var(--ok-fg);
    cursor: pointer;
    transition: transform var(--dur-fast) var(--ease);
  }
  .tick[aria-pressed='true'] {
    border-color: var(--ok-fg);
    transform: scale(0.9);
  }
  .rest {
    position: sticky;
    bottom: 0;
    display: grid;
    grid-template-columns: 1fr auto auto;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-lg);
    background: var(--surface-elevated);
    box-shadow: var(--shadow-md);
    font-variant-numeric: tabular-nums;
  }
  .rest button {
    min-height: 2.5rem;
    padding: 0 var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: none;
    font: inherit;
    color: inherit;
    cursor: pointer;
  }
  /* An activity has no set number, no "last time" and nothing to tick: its line is its words. */
  .wide-what {
    grid-column: 1 / -1;
    text-align: left;
  }
</style>
