<script lang="ts">
  // **The Coach's own calendar** (`decisions.md` 2026-11-20: training shows here and nowhere else).
  // A week or a month: planned and done training by sport; the Menu's planned meals and the
  // read-only meetings as layers the person turns on and off, meetings drawn faintly. A planned
  // session is moved by dragging it to another day or with the date beside it. The Agenda, the board
  // and every other Fitness view are untouched.
  import { untrack } from 'svelte';
  import { coach } from '../ipc';
  import { addDays, civilDay, daysFrom, mondayOf } from '../civil';
  import { dayLabel, dayLetter, dayNumber } from '../units';
  import { setPropertiesUndoable } from '../undo.svelte';
  import { figures, movedStart, sportLabel, stateOf } from './coach';
  import type { CoachDay, ObjectMeta } from '../types';

  interface Props {
    today: string;
    /** Changes whenever something was written: ask again. */
    signal?: unknown;
    onsaved: () => void;
    /** A planned session was done. */
    ondone: (e: ObjectMeta) => void;
  }
  let { today, signal, onsaved, ondone }: Props = $props();

  let mode = $state<'week' | 'month'>('week');
  // svelte-ignore state_referenced_locally
  let anchor = $state(today);
  let meals = $state(true);
  let meetings = $state(true);
  let days = $state<CoachDay[]>([]);
  let failed = $state('');
  let picked = $state<ObjectMeta | null>(null);
  let moveTo = $state('');

  const from = $derived(mode === 'week' ? mondayOf(anchor) : mondayOf(`${anchor.slice(0, 7)}-01`));
  const to = $derived(addDays(from, mode === 'week' ? 6 : 41));
  const month = $derived(anchor.slice(0, 7));

  let asked = 0;
  async function load() {
    const mine = ++asked;
    try {
      const a = await coach(today, from, to);
      if (mine === asked) {
        days = a.days;
        failed = '';
      }
    } catch (e) {
      if (mine === asked) failed = String(e);
    }
  }
  $effect(() => {
    void signal;
    void from;
    void to;
    untrack(() => void load());
  });

  function go(by: number) {
    picked = null;
    anchor =
      mode === 'week'
        ? addDays(anchor, 7 * by)
        : civilDay(Number(anchor.slice(0, 4)), Number(anchor.slice(5, 7)) - 1 + by, 1);
  }

  async function move(e: ObjectMeta, day: string) {
    if (!day || day === (e.start ?? '').slice(0, 10)) return;
    try {
      await setPropertiesUndoable(
        e.id,
        { start: movedStart(e.start, day) },
        `moved “${e.title ?? 'a session'}” to ${dayLabel(day)}`,
      );
      picked = null;
      onsaved();
      await load();
    } catch (err) {
      failed = String(err);
    }
  }
  async function skip(e: ObjectMeta) {
    try {
      const on = stateOf(e) !== 'skipped';
      await setPropertiesUndoable(
        e.id,
        { skipped: on ? 1 : '' },
        `${on ? 'skipped' : 'took back skipping'} “${e.title ?? 'a session'}”`,
      );
      picked = null;
      onsaved();
      await load();
    } catch (err) {
      failed = String(err);
    }
  }

  // ── dragging a planned session to another day ──
  let dragging: ObjectMeta | null = null;
  function drop(ev: DragEvent, day: string) {
    ev.preventDefault();
    const e = dragging;
    dragging = null;
    if (e) void move(e, day);
  }

  /// The sessions of a day: one line per activity, and a day's sets as one line.
  function lines(
    d: CoachDay,
  ): { key: string; e: ObjectMeta | null; text: string; state: string; sport: string }[] {
    const out = [];
    const sets = d.training.filter((e) => e.props.activity === undefined);
    for (const e of d.training.filter((x) => x.props.activity !== undefined))
      out.push({
        key: e.id,
        e,
        text: e.title ?? String(e.props.activity),
        state: stateOf(e),
        sport: String(e.props.sport ?? ''),
      });
    if (sets.length)
      out.push({
        key: `sets:${d.day}`,
        e: null,
        text: `${sets.length} ${sets.length === 1 ? 'set' : 'sets'}`,
        state: sets.every((s) => s.props.planned) ? 'planned' : 'done',
        sport: 'strength',
      });
    return out;
  }
</script>

<section class="card" aria-label="Calendar">
  <header>
    <div class="seg" role="group" aria-label="Calendar span">
      <button type="button" class:on={mode === 'week'} onclick={() => (mode = 'week')}>Week</button>
      <button type="button" class:on={mode === 'month'} onclick={() => (mode = 'month')}
        >Month</button
      >
    </div>
    <div class="nav">
      <button type="button" class="round" aria-label="earlier" onclick={() => go(-1)}>‹</button>
      <span>{mode === 'week' ? `${dayLabel(from)} – ${dayLabel(to)}` : month}</span>
      <button type="button" class="round" aria-label="later" onclick={() => go(1)}>›</button>
    </div>
    <div class="layers">
      <label><input type="checkbox" bind:checked={meals} /> Meals</label>
      <label><input type="checkbox" bind:checked={meetings} /> Meetings</label>
    </div>
  </header>
  {#if failed}<p class="failed" role="alert">{failed}</p>{/if}
  <div class="grid" class:month={mode === 'month'}>
    {#each days as d (d.day)}
      <div
        class="day"
        class:today={d.day === today}
        class:other={mode === 'month' && d.day.slice(0, 7) !== month}
        role="region"
        aria-label={dayLabel(d.day)}
        ondragover={(ev) => ev.preventDefault()}
        ondrop={(ev) => drop(ev, d.day)}
      >
        <p class="date">
          {#if mode === 'week'}{dayLabel(d.day)}{:else}<span>{dayLetter(d.day)}</span>
            {dayNumber(d.day)}{/if}
        </p>
        {#each lines(d) as l (l.key)}
          {#if l.e && l.state !== 'done'}
            {@const e = l.e}
            <button
              type="button"
              class="item"
              data-state={l.state}
              data-sport={l.sport}
              draggable="true"
              ondragstart={() => (dragging = e)}
              onclick={() => {
                picked = e;
                moveTo = (e.start ?? '').slice(0, 10);
              }}
              title={figures(e)}>{l.text}</button
            >
          {:else}
            <span class="item" data-state={l.state} data-sport={l.sport}>{l.text}</span>
          {/if}
        {/each}
        {#if meals}
          {#each d.meals as m (m.id)}<span class="item meal">{m.title}</span>{/each}
        {/if}
        {#if meetings}
          {#each d.meetings as m (m.id)}
            <span class="item meeting"
              >{(m.start ?? '').length > 10 ? `${m.start?.slice(11, 16)} ` : ''}{m.title}</span
            >
          {/each}
        {/if}
      </div>
    {/each}
  </div>
  {#if picked}
    {@const e = picked}
    <div class="picked" role="group" aria-label="the chosen session">
      <p>
        <strong>{e.title}</strong> · {sportLabel(e.props.sport)}{figures(e)
          ? ` · ${figures(e)}`
          : ''}
        {#if daysFrom(today, (e.start ?? '').slice(0, 10)) < 0}<span class="note">(past)</span>{/if}
      </p>
      <label>
        Move to
        <input type="date" bind:value={moveTo} />
      </label>
      <button type="button" class="quiet" onclick={() => move(e, moveTo)}>Move</button>
      <button
        type="button"
        class="quiet"
        onclick={() => {
          picked = null;
          ondone(e);
        }}>Done</button
      >
      <button type="button" class="quiet" onclick={() => skip(e)}
        >{stateOf(e) === 'skipped' ? 'Not skipped' : 'Skip'}</button
      >
      <button type="button" class="quiet" onclick={() => (picked = null)}>Close</button>
    </div>
  {/if}
</section>

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
  header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }
  p {
    margin: 0;
  }
  .seg {
    display: flex;
    padding: 2px;
    border-radius: var(--radius-pill);
    background: var(--surface-hover);
  }
  .seg button {
    min-height: 2.25rem;
    padding: 0 var(--space-3);
    border: 0;
    border-radius: var(--radius-pill);
    background: none;
    color: var(--text-muted);
    font: inherit;
    cursor: pointer;
  }
  .seg button.on {
    background: var(--surface-elevated);
    color: var(--text);
  }
  .nav,
  .layers {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-sm);
  }
  .round {
    width: 2.5rem;
    height: 2.5rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    background: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr;
    gap: var(--space-1);
  }
  @media (min-width: 40rem) {
    .grid {
      grid-template-columns: repeat(7, minmax(0, 1fr));
    }
  }
  .grid.month {
    grid-template-columns: repeat(7, minmax(0, 1fr));
  }
  .day {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-height: 3rem;
    padding: var(--space-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    overflow: hidden;
  }
  .day.today {
    border-color: var(--accent);
  }
  .day.other {
    opacity: 0.5;
  }
  .date {
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .item {
    display: block;
    width: 100%;
    padding: 1px var(--space-1);
    border: 1px solid transparent;
    border-radius: var(--radius-sm, 4px);
    background: var(--surface-hover);
    color: inherit;
    font: inherit;
    font-size: var(--text-sm);
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  button.item {
    cursor: grab;
  }
  .item[data-state='planned'] {
    border-style: dashed;
    border-color: var(--accent);
    background: none;
  }
  .item[data-state='skipped'] {
    text-decoration: line-through;
    color: var(--text-muted);
    background: none;
  }
  .item[data-state='done'] {
    background: var(--accent);
    color: var(--accent-fg, var(--bg));
  }
  .item[data-sport='strength'][data-state='done'] {
    background: var(--text-muted);
  }
  .item.meal {
    background: none;
    border-color: var(--border);
    color: var(--text-muted);
  }
  .item.meeting {
    background: none;
    opacity: 0.55;
    font-style: italic;
  }
  .picked {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: var(--space-2);
    padding-top: var(--space-2);
    border-top: 1px solid var(--border);
  }
  .picked p {
    flex: 1 1 100%;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--text-sm);
  }
  .layers label {
    flex-direction: row;
    align-items: center;
  }
  input[type='date'] {
    min-height: 2.5rem;
    font: inherit;
  }
  .quiet {
    min-height: 2.5rem;
    padding: 0 var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    background: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  .note {
    color: var(--text-muted);
  }
  .failed {
    color: var(--danger-fg);
    font-size: var(--text-sm);
  }
</style>
