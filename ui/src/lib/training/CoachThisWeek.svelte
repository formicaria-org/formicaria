<script lang="ts">
  // **This week and next week** of the plan followed (Spec K.2). This week's sessions as rows the
  // person fills — a published plan's own figures where it gives them, blanks otherwise — and one
  // press puts them on the calendar as planned entries linked to the plan. Beside it, next week's
  // template next to what was done this week; **Next week** moves the plan on, and only that press
  // does (`decisions.md` 2026-11-20: the person steps a plan forward, never the app).
  import { untrack } from 'svelte';
  import WorkoutOut from './WorkoutOut.svelte';
  import { createNotesUndoable, setPropertiesUndoable } from '../undo.svelte';
  import { dayLabel } from '../units';
  import {
    draftsFor,
    figures,
    rowsFor,
    SPORTS,
    sportLabel,
    stateOf,
    typeLabel,
    weekOf,
    type WeekRow,
  } from './coach';
  import type { CoachFollowed, ObjectMeta } from '../types';

  interface Props {
    followed: CoachFollowed;
    today: string;
    /** This calendar week's training entries. */
    entries: ObjectMeta[];
    onsaved: () => void;
  }
  let { followed, today, entries, onsaved }: Props = $props();

  const plan = $derived(followed.plan);
  const week = $derived(followed.week);
  const template = $derived(plan?.weeks?.[week - 1]);
  const next = $derived(plan?.weeks?.[week]);
  const days = $derived(weekOf(today));
  /// This plan's entries on this week's calendar, for its current week.
  const mine = $derived(
    entries.filter((e) => e.props.plan === followed.note.id && e.props.plan_week === week),
  );
  const done = $derived(mine.filter((e) => stateOf(e) === 'done').length);

  let rows = $state<WeekRow[]>([]);
  let busy = $state(false);
  let failed = $state('');
  // The rows start again from the template when the week or the plan changes — not on every refresh,
  // which would throw away what the person was typing.
  let filled = '';
  $effect(() => {
    const key = `${followed.note.id}:${week}:${days[0]}`;
    if (key === filled || !template) return;
    filled = key;
    rows = untrack(() => rowsFor(template, days[0]));
  });

  async function put() {
    if (busy || !rows.length) return;
    busy = true;
    failed = '';
    try {
      await createNotesUndoable(
        draftsFor(rows, followed.note.id, week),
        `put week ${week} of ${plan?.name ?? 'the plan'} on the calendar`,
      );
      onsaved();
    } catch (e) {
      failed = String(e);
    } finally {
      busy = false;
    }
  }

  async function step() {
    if (busy) return;
    busy = true;
    failed = '';
    try {
      await setPropertiesUndoable(
        followed.note.id,
        { week: week + 1 },
        `moved ${plan?.name ?? 'the plan'} to week ${week + 1}`,
      );
      onsaved();
    } catch (e) {
      failed = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<section class="card" aria-label="This week">
  <header>
    <h3>Week {week}{plan?.weeks ? ` of ${plan.weeks.length}` : ''}</h3>
    {#if template?.note}<p class="note">{template.note}</p>{/if}
  </header>
  {#if !plan}
    <p class="note">This version does not carry the plan this note follows.</p>
  {:else if !template}
    <p>The plan's last week is behind you.</p>
  {:else if mine.length}
    <p>This week is on your calendar: {done} of {mine.length} done.</p>
    <ul class="entries">
      {#each mine as e (e.id)}
        <li data-state={stateOf(e)}>
          <span>{dayLabel((e.start ?? '').slice(0, 10))}</span>
          <span>{e.title}</span>
          <span class="note">{figures(e)}</span>
          {#if stateOf(e) === 'planned' && e.props.activity !== undefined}
            <details class="out">
              <summary>To the watch</summary>
              <WorkoutOut id={e.id} />
            </details>
          {/if}
        </li>
      {/each}
    </ul>
  {:else}
    {#if rows.some((r) => r.numbers_from === 'person')}
      <p class="note">
        Fill in your own minutes, distances and targets: the plan leaves them to you. Leave a field
        empty to decide on the day.
      </p>
    {/if}
    <ol class="rows">
      {#each rows as r, i (r.session)}
        <li>
          <p><strong>{typeLabel(r.type)}</strong> — {r.said}</p>
          <div class="fields">
            <label>
              Day
              <select bind:value={rows[i].day}>
                {#each days as d (d)}<option value={d}>{dayLabel(d)}</option>{/each}
              </select>
            </label>
            <label>
              Sport
              <select bind:value={rows[i].sport}>
                {#each SPORTS as s (s.value)}<option value={s.value}>{s.label}</option>{/each}
              </select>
            </label>
            <label>
              Minutes
              <input type="number" min="0" inputmode="numeric" bind:value={rows[i].minutes} />
            </label>
            {#if r.sport !== 'strength'}
              <label>
                Kilometres
                <input
                  type="number"
                  min="0"
                  step="0.1"
                  inputmode="decimal"
                  bind:value={rows[i].km}
                />
              </label>
            {/if}
            <label class="wide">
              Target
              <input type="text" placeholder="e.g. an effort, a pace" bind:value={rows[i].target} />
            </label>
            <label class="wide">
              Steps
              <textarea rows="3" bind:value={rows[i].steps}></textarea>
            </label>
          </div>
        </li>
      {/each}
    </ol>
    <button type="button" class="primary" disabled={busy} onclick={put}>
      Put this week on my calendar
    </button>
  {/if}
  {#if failed}<p class="failed" role="alert">{failed}</p>{/if}
</section>

{#if plan && template}
  <section class="card" aria-label="Next week">
    <h3>Next week</h3>
    {#if next}
      <div class="side">
        <div>
          <h4>The plan's week {next.n}</h4>
          {#if next.note}<p class="note">{next.note}</p>{/if}
          <ul>
            {#each next.sessions as s (s.id)}
              <li>{typeLabel(s.type)} · {sportLabel(s.sport)} — {s.said}</li>
            {/each}
          </ul>
        </div>
        <div>
          <h4>What you did this week</h4>
          {#if mine.length}
            <ul>
              {#each mine as e (e.id)}
                <li data-state={stateOf(e)}>
                  {e.title}: {stateOf(e) === 'done'
                    ? 'done'
                    : stateOf(e) === 'skipped'
                      ? 'skipped'
                      : 'not yet'}{figures(e) ? ` (${figures(e)})` : ''}
                </li>
              {/each}
            </ul>
          {:else}
            <p class="note">Nothing of this plan on the calendar this week.</p>
          {/if}
        </div>
      </div>
      <p class="note">
        formicaria does not move the plan on by itself: repeat this week, or go on when you choose.
      </p>
      <button type="button" class="quiet" disabled={busy} onclick={step}>Next week</button>
    {:else}
      <p class="note">This is the plan's last week.</p>
    {/if}
  </section>
{/if}

<style>
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
  }
  .note {
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
  .rows {
    list-style: none;
    padding: 0;
    gap: var(--space-3);
  }
  .rows > li {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding-bottom: var(--space-2);
    border-bottom: 1px solid var(--border);
  }
  .entries {
    list-style: none;
    padding: 0;
  }
  .entries li {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  .entries .out {
    flex-basis: 100%;
    font-size: var(--text-sm);
  }
  [data-state='done'] {
    color: var(--text);
  }
  [data-state='skipped'] {
    text-decoration: line-through;
    color: var(--text-muted);
  }
  .fields {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  label {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--text-sm);
  }
  label.wide {
    flex: 1 1 100%;
  }
  input,
  select,
  textarea {
    min-height: 2.5rem;
    font: inherit;
  }
  input[type='number'] {
    width: 6rem;
  }
  .side {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(14rem, 1fr));
    gap: var(--space-3);
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
