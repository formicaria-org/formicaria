<script lang="ts">
  // A week of training as a timetable: each day, what is meant for it, what was done, and above
  // them what the regime followed asks of the week (`decisions.md#ui`, 2026-10-12).
  //
  // **It writes nothing of its own kind.** What is meant for a weekday is a routine's own
  // `weekdays`; what was done is the day's entries; planning a routine onto a day is the same
  // write the session screen makes. Pressing a day opens that day's session.
  import Icon from '../Icon.svelte';
  import ExampleWeek from '../plan/ExampleWeek.svelte';
  import { logDays, logRoutines, planLibrary, planProgress } from '../ipc';
  import { shownAsk, shownValue, verdict } from '../plan/measure';
  import { addDays, dayLabel, showWith, whole } from '../units';
  import type {
    DayTotals,
    ExampleWeeks,
    NoteDraft,
    PlanProgress,
    Routine,
    TotalsDef,
  } from '../types';
  import { createNotesUndoable } from '../undo.svelte';

  interface Props {
    view: string;
    def: TotalsDef;
    today: string;
    /** Changes whenever something was written: ask again. */
    signal?: unknown;
    /** Open a day's session. */
    onday: (day: string) => void;
    /** Plan a routine's sets onto a day. */
    onplan: (r: Routine, day: string) => void;
    /** Something was written from here. */
    onsaved: () => void;
  }
  let { view, def, today, signal, onday, onplan, onsaved }: Props = $props();

  const mondayOf = (day: string) => addDays(day, -((new Date(`${day}T12:00:00`).getDay() + 6) % 7));
  // svelte-ignore state_referenced_locally
  let monday = $state(mondayOf(today));
  const sunday = $derived(addDays(monday, 6));
  const thisWeek = $derived(monday === mondayOf(today));

  let days = $state<DayTotals[]>([]);
  /// The week before the one shown: what its total came to, and what "the same again" copies.
  let before = $state<DayTotals[]>([]);
  let routines = $state<Routine[]>([]);
  let progress = $state<PlanProgress | null>(null);
  let failed = $state('');
  /// Weeks a public body lays out day by day — shown, word for word, to someone with no timetable
  /// of their own yet. Sunday to Saturday, as their source lists them.
  let examples = $state<ExampleWeeks[]>([]);
  $effect(() => {
    planLibrary('fitness')
      .then((lib) => (examples = lib.weeks ?? []))
      .catch(() => (examples = []));
  });
  const SUNDAY_FIRST = [
    'Sunday',
    'Monday',
    'Tuesday',
    'Wednesday',
    'Thursday',
    'Friday',
    'Saturday',
  ];
  let asked = 0;
  async function load() {
    const mine = ++asked;
    // The week is measured up to today while it is under way, to its end once it is over, and
    // from its start when it is still to come.
    const upto = thisWeek ? today : sunday < today ? sunday : monday;
    try {
      const [d, b, r, p] = await Promise.all([
        logDays(view, monday, sunday, true),
        logDays(view, addDays(monday, -7), addDays(monday, -1), true),
        logRoutines(view),
        planProgress('fitness', upto).catch(() => null),
      ]);
      if (mine !== asked) return;
      days = d.days;
      before = b.days;
      routines = r;
      progress = p;
      failed = '';
    } catch (e) {
      if (mine === asked) failed = String(e);
    }
  }
  $effect(() => {
    void signal;
    void monday;
    void load();
  });

  const WEEK = ['sun', 'mon', 'tue', 'wed', 'thu', 'fri', 'sat'];
  const meantFor = (day: string) => {
    const key = WEEK[new Date(`${day}T12:00:00`).getDay()];
    return routines.filter((r) => r.weekdays.includes(key));
  };
  /// A day in a few words: its sets, its activities, and which exercises.
  function did(d: DayTotals) {
    const rows = d.rows ?? [];
    const sets = rows.filter((r) => r.props.activity === undefined);
    const acts = rows.filter((r) => r.props.activity !== undefined);
    return {
      sets: sets.length,
      ticked: sets.filter((s) => !s.props.planned).length,
      exercises: [...new Set(sets.map((s) => String(s.props.exercise ?? s.title ?? '')))].filter(
        Boolean,
      ),
      acts: acts.map((a) => `${a.title ?? a.props.activity} · ${whole(a.props.minutes) ?? 0} min`),
    };
  }
  const weekOver = $derived(sunday < today);

  /// What the week's first value came to — the load lifted, for the shipped view — beside the
  /// week before. A total of what was done, never a target.
  const primary = $derived(def.metrics.find((m) => m.role === 'primary') ?? def.metrics[0]);
  const totalOf = (list: DayTotals[]) =>
    list.reduce((t, d) => t + (d.sums[primary.key]?.sum ?? 0), 0);
  const lifted = $derived(totalOf(days));
  const liftedBefore = $derived(totalOf(before));

  /// **The same again**: what was done on each day of last week, planned on the same weekday of
  /// this one — for the days still to come that have nothing on them yet. Sets as they were
  /// done; an activity is logged when it is done, so it is not copied.
  const again = $derived.by(() => {
    const drafts: NoteDraft[] = [];
    before.forEach((d, i) => {
      const target = addDays(monday, i);
      if (target < today || (days[i]?.rows ?? []).length) return;
      for (const r of d.rows ?? []) {
        if (r.props.planned || r.props.activity !== undefined) continue;
        const { work_g: _w, session: _s, ...keep } = r.props;
        const props = Object.fromEntries(
          Object.entries(keep).filter(
            (e): e is [string, string | number] =>
              typeof e[1] === 'string' || typeof e[1] === 'number',
          ),
        );
        drafts.push({ title: r.title ?? '', start: target, props: { ...props, planned: 1 } });
      }
    });
    return drafts;
  });
  let busy = $state(false);
  async function repeat() {
    if (busy || !again.length) return;
    busy = true;
    try {
      await createNotesUndoable(again, 'planned this week as the last one went');
      onsaved();
      await load();
    } catch (e) {
      failed = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="week">
  <header class="head">
    <button
      type="button"
      class="round"
      onclick={() => (monday = addDays(monday, -7))}
      aria-label="the week before"
    >
      <Icon name="chevronLeft" size={18} />
    </button>
    <h2>{thisWeek ? 'This week' : `Week of ${dayLabel(monday)}`}</h2>
    <button
      type="button"
      class="round flip"
      onclick={() => (monday = addDays(monday, 7))}
      aria-label="the week after"
    >
      <Icon name="chevronLeft" size={18} />
    </button>
  </header>
  {#if !thisWeek}
    <button type="button" class="link" onclick={() => (monday = mondayOf(today))}>
      Back to this week
    </button>
  {/if}
  {#if failed}<p class="failed" role="alert">{failed}</p>{/if}

  <section class="card" aria-label="what the week is held to">
    {#if progress?.plan}
      <h3>{progress.plan.name}</h3>
      <ul class="asks">
        {#each progress.checks as c (c.id)}
          <li>
            <span>{c.label}</span>
            <span class="val"
              >{c.value !== undefined && c.value !== null ? shownValue(c, c.value) : '—'}
              <span class="sub">· {shownAsk(c)} · {verdict(c, weekOver)}</span></span
            >
          </li>
        {/each}
      </ul>
    {:else}
      <p class="says">
        Nothing is followed. Choose a regime under Regime and this week is measured against it.
      </p>
    {/if}
  </section>

  <!-- Following something, and no timetable yet: its week to start from, on the first screen. -->
  {#if progress?.plan?.examples?.length}
    {#if routines.length}
      <!-- A timetable of the person's own is there already: the example steps back. -->
      <details class="fold">
        <summary>A week to start from, for {progress.plan.name}</summary>
        {#each progress.plan.examples as example, i (i)}
          <ExampleWeek {example} onsaved={() => (onsaved(), void load())} />
        {/each}
      </details>
    {:else}
      {#each progress.plan.examples as example, i (i)}
        <ExampleWeek {example} onsaved={() => (onsaved(), void load())} />
      {/each}
    {/if}
  {/if}
  {#if lifted > 0 || liftedBefore > 0}
    <p class="says">
      {primary.label} this week: {showWith(primary.key, lifted, 1000)}{#if liftedBefore > 0}
        · the week before: {showWith(primary.key, liftedBefore, 1000)}{/if}
    </p>
  {/if}
  {#if thisWeek && again.length}
    <button type="button" class="quiet wide" onclick={repeat} disabled={busy}>
      Plan the days left as last week went
    </button>
  {/if}

  {#each days as d (d.day)}
    {@const meant = meantFor(d.day)}
    {@const done = did(d)}
    <section class="day" class:today={d.day === today} aria-label={dayLabel(d.day)}>
      <header>
        <h3>{d.day === today ? `Today · ${dayLabel(d.day)}` : dayLabel(d.day)}</h3>
        <button
          type="button"
          class="quiet"
          onclick={() => onday(d.day)}
          aria-label={d.day === today ? 'open today' : `open ${dayLabel(d.day)}`}>Open</button
        >
      </header>
      {#if meant.length}
        <p class="meant">Meant for it: {meant.map((r) => r.title).join(', ')}</p>
      {/if}
      {#if done.sets}
        <p>
          {done.ticked} of {done.sets}
          {done.sets === 1 ? 'set' : 'sets'} done · {done.exercises.join(', ')}
        </p>
      {/if}
      {#each done.acts as a (a)}<p>{a}</p>{/each}
      {#if !done.sets && !done.acts.length}
        {#if meant.length && d.day >= today}
          <div class="plans">
            {#each meant as r (r.id)}
              <button type="button" class="go" onclick={() => onplan(r, d.day)}>
                Plan {r.title}
              </button>
            {/each}
          </div>
        {:else}
          <p class="says">{d.day < today ? 'Nothing logged.' : 'Nothing meant for it.'}</p>
        {/if}
      {/if}
    </section>
  {/each}

  {#if !routines.length}
    <p class="says">
      A timetable comes from routines. On the Session tab, add what you do and keep it as a routine
      for the days you mean to do it.
    </p>
  {/if}

  {#each progress?.plan ? [] : examples as set (set.url)}
    <section class="card" aria-label="example weeks">
      <h3>Example weeks</h3>
      <p class="says">
        As published by the {set.by}, to start from. They meet the weekly activity guidelines; they
        are examples, not a plan made for you.
      </p>
      {#each set.examples as ex (ex.name)}
        <details>
          <summary>{ex.name}: {ex.about}</summary>
          <ul class="asks">
            {#each ex.days as what, i (i)}
              <li><span>{SUNDAY_FIRST[i]}</span><span class="val">{what || '—'}</span></li>
            {/each}
          </ul>
          <p class="says">{ex.total}</p>
        </details>
      {/each}
      <p class="says">
        Source: <a href={set.url} target="_blank" rel="noreferrer noopener">{set.title}</a>, {set.by}
        (seen {set.seen}).
      </p>
    </section>
  {/each}
</div>

<style>
  /* A link is the accent's colour: the browser's own blue cannot be read on a dark theme. */
  a {
    color: var(--accent);
  }
  .week {
    height: 100%;
    overflow-y: auto;
    overscroll-behavior: contain;
    box-sizing: border-box;
    padding: var(--space-3) max(var(--space-3), calc((100% - 36rem) / 2)) var(--space-8);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    scrollbar-width: thin;
  }
  .week > :global(*) {
    flex-shrink: 0;
  }
  .head,
  .day > header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }
  h2 {
    margin: 0;
    font-size: var(--text-xl);
  }
  h3 {
    margin: 0;
    font-size: var(--text-md);
  }
  p {
    margin: 0;
  }
  button {
    font: inherit;
    color: inherit;
  }
  .round {
    flex: none;
    width: 2.75rem;
    height: 2.75rem;
    display: grid;
    place-items: center;
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    background: none;
    cursor: pointer;
  }
  .flip :global(svg) {
    transform: scaleX(-1);
  }
  .card,
  .day {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }
  .day.today {
    border-color: var(--accent);
  }
  .asks {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  .asks li {
    display: flex;
    justify-content: space-between;
    gap: var(--space-2);
    font-size: var(--text-sm);
  }
  .val {
    text-align: right;
  }
  .sub,
  .says,
  .meant {
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .fold {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  details {
    border-top: 1px solid var(--border);
    padding-top: var(--space-2);
  }
  summary {
    min-height: 2.25rem;
    cursor: pointer;
    font-size: var(--text-sm);
  }
  .plans {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  .go,
  .quiet {
    min-height: 2.75rem;
    padding: 0 var(--space-3);
    border-radius: var(--radius-md);
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
  }
  .wide {
    align-self: stretch;
  }
  .link {
    align-self: flex-start;
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent);
    cursor: pointer;
  }
  .failed {
    font-size: var(--text-sm);
    color: var(--danger-fg);
  }
</style>
