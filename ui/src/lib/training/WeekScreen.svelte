<script lang="ts">
  // A week of training as a timetable: each day, what is meant for it, what was done, and above
  // them what the regime followed asks of the week (`decisions.md#ui`, 2026-10-12).
  //
  // **It writes nothing of its own kind.** What is meant for a weekday is a routine's own
  // `weekdays`; what was done is the day's entries; planning a routine onto a day is the same
  // write the session screen makes. Pressing a day opens that day's session.
  import Icon from '../Icon.svelte';
  import { logDays, logRoutines, planProgress } from '../ipc';
  import { shownAsk, shownValue, verdict } from '../plan/measure';
  import { addDays, dayLabel, whole } from '../units';
  import type { DayTotals, PlanProgress, Routine } from '../types';

  interface Props {
    view: string;
    today: string;
    /** Changes whenever something was written: ask again. */
    signal?: unknown;
    /** Open a day's session. */
    onday: (day: string) => void;
    /** Plan a routine's sets onto a day. */
    onplan: (r: Routine, day: string) => void;
  }
  let { view, today, signal, onday, onplan }: Props = $props();

  const mondayOf = (day: string) => addDays(day, -((new Date(`${day}T12:00:00`).getDay() + 6) % 7));
  // svelte-ignore state_referenced_locally
  let monday = $state(mondayOf(today));
  const sunday = $derived(addDays(monday, 6));
  const thisWeek = $derived(monday === mondayOf(today));

  let days = $state<DayTotals[]>([]);
  let routines = $state<Routine[]>([]);
  let progress = $state<PlanProgress | null>(null);
  let failed = $state('');
  let asked = 0;
  async function load() {
    const mine = ++asked;
    // The week is measured up to today while it is under way, to its end once it is over, and
    // from its start when it is still to come.
    const upto = thisWeek ? today : sunday < today ? sunday : monday;
    try {
      const [d, r, p] = await Promise.all([
        logDays(view, monday, sunday, true),
        logRoutines(view),
        planProgress('fitness', upto).catch(() => null),
      ]);
      if (mine !== asked) return;
      days = d.days;
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

  {#each days as d (d.day)}
    {@const meant = meantFor(d.day)}
    {@const done = did(d)}
    <section class="day" class:today={d.day === today} aria-label={dayLabel(d.day)}>
      <header>
        <h3>{d.day === today ? `Today · ${dayLabel(d.day)}` : dayLabel(d.day)}</h3>
        <button type="button" class="quiet" onclick={() => onday(d.day)}>Open</button>
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
</div>

<style>
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
