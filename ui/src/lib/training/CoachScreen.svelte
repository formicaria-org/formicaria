<script lang="ts">
  // **Fitness → Coach** (`decisions.md` 2026-11-20, *Fitness gets a Coach tab; the app still chooses
  // no loads*; the coach plan's Spec K). Top to bottom: Today, This week and next, the Coach's own
  // calendar, Progress, and the Plans — the library first when nothing is followed.
  //
  // It asks for its own data (`coach`, `match_planned`) and writes only with the commands every
  // tracker writes with, each undoable; the one exception is accepting a watch match, which the
  // core records as one step and undo takes back. **Nothing here moves a plan on or fills in a
  // number**: the person puts a week on the calendar and presses Next week.
  import { untrack } from 'svelte';
  import { coach, getNote, matchPlanned, planLibrary } from '../ipc';
  import { acceptMatchUndoable, setPropertiesUndoable } from '../undo.svelte';
  import { dayLabel } from '../units';
  import CoachPlans from './CoachPlans.svelte';
  import CoachThisWeek from './CoachThisWeek.svelte';
  import CoachCalendar from './CoachCalendar.svelte';
  import CoachProgress from './CoachProgress.svelte';
  import WorkoutOut from './WorkoutOut.svelte';
  import { figures, movedStart, sportLabel, stateOf, weekOf } from './coach';
  import type { CoachAnswer, CoachMatches, ObjectMeta, PlanOption } from '../types';

  interface Props {
    today: string;
    /** Anything the app says changed: ask again. */
    signal?: unknown;
    onsaved: () => void;
    /** A planned session was done: the pane's own confirm, which keeps what was planned. */
    ondone: (e: ObjectMeta) => void;
    /** Open a day's session (its sets to tick) — the Week's day screen. */
    onday: (day: string) => void;
  }
  let { today, signal, onsaved, ondone, onday }: Props = $props();

  let answer = $state<CoachAnswer | null>(null);
  let plans = $state<PlanOption[]>([]);
  let matches = $state<CoachMatches | null>(null);
  let failed = $state('');
  /// Each of today's planned sessions' steps, as written in its note.
  let steps = $state<Record<string, string>>({});
  /// Proposals the person said no to, for as long as the screen is open.
  let declined = $state<string[]>([]);
  /// The session whose Move is open, and the day typed for it.
  let moving = $state<string | null>(null);
  let moveDay = $state('');

  const week = $derived(weekOf(today));
  let asked = 0;
  async function load() {
    const mine = ++asked;
    try {
      const [a, lib, m] = await Promise.all([
        coach(today, week[0], week[6]),
        planLibrary('fitness'),
        matchPlanned().catch(() => null),
      ]);
      if (mine !== asked) return;
      answer = a;
      plans = lib.plans.filter((p) => p.kind === 'coach');
      matches = m;
      failed = '';
    } catch (e) {
      if (mine === asked) failed = String(e);
    }
  }
  $effect(() => {
    void signal;
    untrack(() => void load());
  });

  const followed = $derived(answer?.followed ?? []);
  const current = $derived(followed[0]);
  const thisWeek = $derived((answer?.days ?? []).flatMap((d) => d.training));
  const todays = $derived(
    (answer?.days ?? [])
      .find((d) => d.day === today)
      ?.training.filter((e) => e.props.activity !== undefined) ?? [],
  );
  const todaysSets = $derived(
    (answer?.days ?? [])
      .find((d) => d.day === today)
      ?.training.filter((e) => e.props.activity === undefined) ?? [],
  );
  $effect(() => {
    for (const e of todays) {
      if (e.id in untrack(() => steps)) continue;
      void getNote(e.id)
        .then((n) => (steps = { ...steps, [e.id]: n?.body ?? '' }))
        .catch(() => undefined);
    }
  });
  const proposals = $derived(
    (matches?.proposals ?? []).filter((p) => !declined.includes(p.imported.id)),
  );

  async function write(fn: () => Promise<void>) {
    failed = '';
    try {
      await fn();
      onsaved();
      await load();
    } catch (e) {
      failed = String(e);
    }
  }
  const skip = (e: ObjectMeta) =>
    write(() =>
      setPropertiesUndoable(
        e.id,
        { skipped: stateOf(e) === 'skipped' ? '' : 1 },
        `${stateOf(e) === 'skipped' ? 'took back skipping' : 'skipped'} “${e.title ?? 'a session'}”`,
      ),
    );
  const move = (e: ObjectMeta, day: string) =>
    write(async () => {
      if (!day) return;
      await setPropertiesUndoable(
        e.id,
        { start: movedStart(e.start, day) },
        `moved “${e.title ?? 'a session'}” to ${dayLabel(day)}`,
      );
      moving = null;
    });
  const accept = (planned: ObjectMeta, imported: ObjectMeta) =>
    write(async () => {
      await acceptMatchUndoable(
        planned.id,
        imported.id,
        `kept the watch's session as “${planned.title ?? 'the planned session'}”`,
      );
    });
  function done(e: ObjectMeta) {
    ondone(e);
  }
</script>

<div class="coach">
  {#if failed}<p class="failed" role="alert">{failed}</p>{/if}

  {#if proposals.length}
    <section class="card" aria-label="From your watch">
      {#each proposals as p (p.imported.id)}
        <div class="proposal">
          <p>
            This was your planned <strong>{p.planned.title}</strong> on {dayLabel(
              (p.planned.start ?? '').slice(0, 10),
            )}? The watch recorded {figures(p.imported) || 'a session'}.
          </p>
          <div class="row">
            <button type="button" class="primary" onclick={() => accept(p.planned, p.imported)}
              >Yes, it was</button
            >
            <button
              type="button"
              class="quiet"
              onclick={() => (declined = [...declined, p.imported.id])}>No</button
            >
          </div>
        </div>
      {/each}
    </section>
  {/if}
  {#if matches?.unmatched.length}
    <section class="card" aria-label="Watch sessions to place">
      <p class="note">
        {matches.unmatched.length === 1 ? 'A session' : `${matches.unmatched.length} sessions`} from your
        watch could be more than one planned session; open the day to sort them out.
      </p>
      <ul>
        {#each matches.unmatched as u (u.imported.id)}
          <li>
            <button
              type="button"
              class="link"
              onclick={() =>
                onday(String(u.imported.props.day ?? u.imported.start ?? '').slice(0, 10))}
              >{u.imported.title} — {dayLabel(
                String(u.imported.props.day ?? u.imported.start ?? '').slice(0, 10),
              )}</button
            >
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  {#if answer && !current}
    <section class="card intro">
      <h2>Coach</h2>
      <p>
        Plans for a goal with a date or a season. Pick one to follow: published plans show their
        source's own figures; assembled plans leave every number to you. formicaria never moves a
        plan to its next week — you do.
      </p>
    </section>
    <CoachPlans {plans} {today} {followed} onsaved={() => void write(async () => {})} />
  {/if}

  {#if current}
    <section class="card" aria-label="Today">
      <h2>Today</h2>
      {#if !todays.length && !todaysSets.length}
        <p class="note">Nothing planned for today.</p>
      {/if}
      <ul class="today">
        {#each todays as e (e.id)}
          <li data-state={stateOf(e)}>
            <p>
              <strong>{e.title}</strong> · {sportLabel(e.props.sport)}{figures(e)
                ? ` · ${figures(e)}`
                : ''}
              {#if stateOf(e) === 'done'}<span class="badge">Done</span>{/if}
              {#if stateOf(e) === 'skipped'}<span class="badge">Skipped</span>{/if}
            </p>
            {#if steps[e.id]}<pre class="steps">{steps[e.id]}</pre>{/if}
            {#if stateOf(e) === 'planned'}<WorkoutOut id={e.id} />{/if}
            {#if stateOf(e) !== 'done'}
              <div class="row">
                {#if stateOf(e) === 'planned'}
                  <button type="button" class="primary" onclick={() => done(e)}>Done</button>
                {/if}
                {#if moving === e.id}
                  <label>
                    Move to
                    <input type="date" bind:value={moveDay} />
                  </label>
                  <button type="button" class="quiet" onclick={() => move(e, moveDay)}>Move</button>
                {:else}
                  <button
                    type="button"
                    class="quiet"
                    onclick={() => {
                      moving = e.id;
                      moveDay = today;
                    }}>Move</button
                  >
                {/if}
                <button type="button" class="quiet" onclick={() => skip(e)}
                  >{stateOf(e) === 'skipped' ? 'Not skipped' : 'Skip'}</button
                >
              </div>
            {/if}
          </li>
        {/each}
      </ul>
      {#if todaysSets.length}
        <button type="button" class="quiet" onclick={() => onday(today)}>
          Today's sets ({todaysSets.length})
        </button>
      {/if}
    </section>

    <CoachThisWeek
      followed={current}
      {today}
      entries={thisWeek}
      onsaved={() => void write(async () => {})}
    />
    <CoachCalendar {today} {signal} {onsaved} ondone={done} />
    <CoachProgress {today} {signal} />
    <section class="card" aria-label="Plans">
      <h2>Plans</h2>
      <CoachPlans {plans} {today} {followed} onsaved={() => void write(async () => {})} />
    </section>
  {/if}
</div>

<style>
  .coach {
    height: 100%;
    overflow-y: auto;
    overscroll-behavior: contain;
    box-sizing: border-box;
    padding: var(--space-3) max(var(--space-3), calc((100% - 48rem) / 2)) var(--space-8);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    scrollbar-width: thin;
  }
  .coach > :global(*) {
    flex-shrink: 0;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }
  h2 {
    margin: 0;
    font-size: var(--text-xl);
  }
  p {
    margin: 0;
  }
  ul {
    margin: 0;
    padding-left: var(--space-4);
  }
  .today {
    list-style: none;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .today li {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  [data-state='skipped'] p {
    text-decoration: line-through;
    color: var(--text-muted);
  }
  .steps {
    margin: 0;
    white-space: pre-wrap;
    font: inherit;
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: var(--space-2);
  }
  .badge {
    margin-left: var(--space-1);
    padding: 0 var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--surface-hover);
    font-size: var(--text-sm);
  }
  .note {
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  label {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--text-sm);
  }
  input[type='date'] {
    min-height: 2.5rem;
    font: inherit;
  }
  button.quiet,
  button.primary {
    min-height: 2.75rem;
    padding: 0 var(--space-4);
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    background: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  button.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-fg, var(--bg));
  }
  button.link {
    min-height: 2.25rem;
    border: 0;
    background: none;
    color: var(--accent);
    font: inherit;
    cursor: pointer;
    text-align: left;
  }
  .proposal {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .failed {
    color: var(--danger-fg);
    font-size: var(--text-sm);
  }
</style>
