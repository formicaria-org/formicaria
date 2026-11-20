<script lang="ts">
  // The Coach's library: plans for a goal, filtered by goal and sport; one plan's page — its weeks
  // at a glance, its rules with their sources, who it is not for, its caution — and **Follow**,
  // which writes one program note (`decisions.md` 2026-11-20). A published plan is offered by its
  // name and links to its source; an assembled one says it carries no number of its own.
  import { createNotesUndoable, setPropertiesUndoable } from '../undo.svelte';
  import { addDays } from '../units';
  import { PARTS, sportLabel, typeLabel } from './coach';
  import type { CoachFollowed, PlanOption } from '../types';

  interface Props {
    plans: PlanOption[];
    today: string;
    /** The Coach plans in force, the latest first. */
    followed: CoachFollowed[];
    onsaved: () => void;
  }
  let { plans, today, followed, onsaved }: Props = $props();

  let sport = $state('');
  let goal = $state('');
  let open = $state<PlanOption | null>(null);
  let busy = $state(false);
  let failed = $state('');
  /// The follow form: what the person trains for, and the day of the event, if there is one.
  let myGoal = $state('');
  let eventDate = $state('');

  const sports = $derived([...new Set(plans.flatMap((p) => p.sports ?? []))]);
  const goals = $derived([...new Set(plans.map((p) => p.goal ?? '').filter(Boolean))]);
  const shown = $derived(
    plans.filter((p) => (!sport || (p.sports ?? []).includes(sport)) && (!goal || p.goal === goal)),
  );
  const RESTS: Record<string, string> = {
    published: 'A published plan',
    assembled: 'Assembled from research',
  };
  const DAYS: Record<string, string> = {
    mon: 'Mon',
    tue: 'Tue',
    wed: 'Wed',
    thu: 'Thu',
    fri: 'Fri',
    sat: 'Sat',
    sun: 'Sun',
  };
  const isFollowed = (id: string) => followed.some((f) => f.plan?.id === id);

  function look(p: PlanOption) {
    open = p;
    myGoal = p.goal ?? '';
    eventDate = '';
    failed = '';
  }

  /// The note following writes: the plan in its own words and whose they are, and the rule the app
  /// keeps — it never steps a week forward.
  function bodyOf(p: PlanOption): string {
    const lines = [p.summary, ''];
    for (const r of p.rules ?? []) lines.push(`- ${r.said}`);
    lines.push('', 'Sources:');
    for (const s of p.sources) lines.push(`- ${s.by}: ${s.title} ${s.url}`);
    lines.push('', 'formicaria never moves this plan to its next week: you do, with Next week.');
    return lines.join('\n') + '\n';
  }

  async function follow() {
    const p = open;
    if (!p || busy) return;
    busy = true;
    failed = '';
    try {
      // One Coach plan at a time: the one followed so far gets a last day.
      for (const f of followed)
        await setPropertiesUndoable(
          f.note.id,
          { due: addDays(today, -1) },
          `stopped following ${f.note.title ?? 'a plan'}`,
        );
      await createNotesUndoable(
        [
          {
            title: p.name,
            start: today,
            body: bodyOf(p),
            props: {
              program: 'coach',
              plan: p.id,
              question: p.question,
              goal: myGoal.trim() || (p.goal ?? ''),
              week: 1,
              ...(eventDate ? { event_date: eventDate } : {}),
            },
          },
        ],
        `followed ${p.name}`,
      );
      open = null;
      onsaved();
    } catch (e) {
      failed = String(e);
    } finally {
      busy = false;
    }
  }

  async function stop(f: CoachFollowed) {
    if (busy) return;
    busy = true;
    try {
      await setPropertiesUndoable(
        f.note.id,
        { due: addDays(today, -1) },
        `stopped following ${f.note.title ?? 'a plan'}`,
      );
      onsaved();
    } catch (e) {
      failed = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<section class="plans" aria-label="Plans">
  {#if open}
    {@const p = open}
    <article class="card page">
      <button type="button" class="quiet" onclick={() => (open = null)}>‹ All plans</button>
      <h3>{p.name}</h3>
      <p class="badges">
        <span class="badge">{RESTS[p.rests_on] ?? p.rests_on}</span>
        {#if isFollowed(p.id)}<span class="badge on">Following</span>{/if}
      </p>
      {#if p.goal}<p><strong>For:</strong> {p.goal}</p>{/if}
      {#if p.level_in}<p><strong>Starts from:</strong> {p.level_in}</p>{/if}
      <p>{p.summary}</p>
      {#if p.rests_on === 'assembled'}
        <p class="note">
          Assembled by formicaria: the sessions carry no number of their own. You write every
          minute, distance and target.
        </p>
      {:else if p.rests_on === 'published'}
        <p class="note">
          A published plan, offered by its name: its figures are its source's, shown as theirs or
          taken from its page.
        </p>
      {/if}
      {#each p.guides as g (g.url)}
        <p><a href={g.url} target="_blank" rel="noopener noreferrer">{g.label}</a> ({g.by})</p>
      {/each}
      {#if p.rules?.length}
        <h4>What it rests on</h4>
        <ul>
          {#each p.rules as r (r.said)}
            <li>
              {r.said}
              {#if p.sources[r.source]}
                <a href={p.sources[r.source].url} target="_blank" rel="noopener noreferrer"
                  >source</a
                >
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
      {#if p.weeks?.length}
        <h4>Weeks at a glance</h4>
        <ol class="weeks">
          {#each p.weeks as w (w.n)}
            <li>
              <span class="n">Week {w.n}</span>
              <span
                >{w.sessions
                  .map((s) => `${DAYS[s.day_hint]} ${typeLabel(s.type)} (${sportLabel(s.sport)})`)
                  .join(' · ')}</span
              >
            </li>
          {/each}
        </ol>
        {#if p.weeks[0].sessions[0]?.steps.length}
          <p class="note">
            A session's steps, e.g.
            {p.weeks[0].sessions[0].steps
              .map((t) => `${PARTS[t.part] ?? t.part}: ${t.said}`)
              .join('; ')}.
          </p>
        {/if}
      {/if}
      {#if p.not_for_words?.length}
        <h4>Not for</h4>
        <ul>
          {#each p.not_for_words as w (w)}<li>{w}</li>{/each}
        </ul>
      {/if}
      {#if p.caution}<p class="caution" role="note">{p.caution}</p>{/if}
      {#each p.unmeasured as u (u)}<p class="note">{u}</p>{/each}
      <h4>Sources</h4>
      <ul>
        {#each p.sources as s (s.url)}
          <li>
            {s.by}: <a href={s.url} target="_blank" rel="noopener noreferrer">{s.title}</a>
          </li>
        {/each}
      </ul>
      {#if !isFollowed(p.id)}
        <div class="follow">
          <label>
            What you train for
            <input type="text" bind:value={myGoal} />
          </label>
          <label>
            The day of your event (if there is one)
            <input type="date" bind:value={eventDate} />
          </label>
          <button type="button" class="primary" disabled={busy} onclick={follow}>Follow</button>
          {#if followed.length}
            <p class="note">
              Following it stops {followed[0].note.title ?? 'the plan you follow'}.
            </p>
          {/if}
        </div>
      {/if}
      {#if failed}<p class="failed" role="alert">{failed}</p>{/if}
    </article>
  {:else}
    {#each followed as f (f.note.id)}
      <div class="card following">
        <p>
          Following <strong>{f.note.title}</strong>{f.goal ? ` — ${f.goal}` : ''}{f.event_date
            ? `, for ${f.event_date}`
            : ''}.
        </p>
        <div class="row">
          {#if f.plan}
            {@const plan = f.plan}
            <button type="button" class="quiet" onclick={() => look(plan)}>Its page</button>
          {/if}
          <button type="button" class="quiet" disabled={busy} onclick={() => stop(f)}
            >Stop following</button
          >
        </div>
      </div>
    {/each}
    <div class="filters">
      <label>
        Sport
        <select bind:value={sport}>
          <option value="">Any</option>
          {#each sports as s (s)}<option value={s}>{sportLabel(s)}</option>{/each}
        </select>
      </label>
      <label>
        Goal
        <select bind:value={goal}>
          <option value="">Any</option>
          {#each goals as g (g)}<option value={g}>{g}</option>{/each}
        </select>
      </label>
    </div>
    <ul class="list">
      {#each shown as p (p.id)}
        <li>
          <button type="button" class="option" onclick={() => look(p)}>
            <strong>{p.name}</strong>
            <span>{p.goal}</span>
            <span class="badges">
              <span class="badge">{RESTS[p.rests_on] ?? p.rests_on}</span>
              {#if isFollowed(p.id)}<span class="badge on">Following</span>{/if}
            </span>
          </button>
        </li>
      {:else}
        <li class="note">No plan for that sport and goal.</li>
      {/each}
    </ul>
    {#if failed}<p class="failed" role="alert">{failed}</p>{/if}
  {/if}
</section>

<style>
  a {
    color: var(--accent);
  }
  .plans {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
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
  h3,
  h4,
  p {
    margin: 0;
  }
  h4 {
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  ul,
  ol {
    margin: 0;
    padding-left: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  .list {
    list-style: none;
    padding: 0;
  }
  .weeks {
    list-style: none;
    padding: 0;
    font-size: var(--text-sm);
  }
  .weeks .n {
    display: inline-block;
    min-width: 4.5rem;
    color: var(--text-muted);
  }
  .option {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-1);
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .option:hover {
    border-color: var(--accent);
  }
  .badges {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
  }
  .badge {
    padding: 0 var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--surface-hover);
    font-size: var(--text-xs, var(--text-sm));
  }
  .badge.on {
    background: var(--accent);
    color: var(--accent-fg, var(--bg));
  }
  .note {
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .caution {
    font-size: var(--text-sm);
    padding: var(--space-2);
    border-left: 3px solid var(--warning-fg, var(--accent));
  }
  .filters,
  .row,
  .follow {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: var(--space-2);
  }
  label {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--text-sm);
  }
  input,
  select {
    min-height: 2.5rem;
    font: inherit;
  }
  button.quiet,
  button.primary {
    min-height: 2.5rem;
    padding: 0 var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    background: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
    align-self: flex-start;
  }
  button.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-fg, var(--bg));
  }
  .failed {
    color: var(--danger-fg);
    font-size: var(--text-sm);
  }
</style>
