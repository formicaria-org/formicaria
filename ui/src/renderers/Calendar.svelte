<script lang="ts">
  import {
    monthGrid,
    weekOf,
    monthLabel,
    weekLabel,
    WEEKDAYS,
    ymd,
    dayOfMonth,
    addDays,
  } from '../lib/calendar';
  import { urgency } from '../lib/urgency';
  import type { ObjectMeta } from '../lib/types';

  // A renderer over the same agenda cards (dated, open) the list view uses — a
  // month or week grid instead of a list. `range` picks the granularity; a single
  // cursor date drives both. Events are tinted by *derived* urgency, never a
  // stored priority. No status literal appears here.
  let {
    cards,
    onopen,
    range = 'month',
  }: { cards: ObjectMeta[]; onopen: (id: string) => void; range?: 'month' | 'week' } = $props();

  const now = new Date();
  const todayIso = ymd(now);
  let cursor = $state(todayIso); // month view ignores the day-of-month

  let year = $derived(Number(cursor.slice(0, 4)));
  let month = $derived(Number(cursor.slice(5, 7)) - 1);

  let weeks = $derived(range === 'week' ? [weekOf(cursor)] : monthGrid(year, month));
  let label = $derived(range === 'week' ? weekLabel(weekOf(cursor)) : monthLabel(year, month));

  // Bucket cards onto their due day (YYYY-MM-DD).
  let byDay = $derived.by(() => {
    const m = new Map<string, ObjectMeta[]>();
    for (const c of cards) {
      if (!c.due) continue;
      const key = c.due.slice(0, 10);
      const arr = m.get(key);
      if (arr) arr.push(c);
      else m.set(key, [c]);
    }
    return m;
  });

  function prev() {
    cursor = range === 'week' ? addDays(cursor, -7) : ymd(new Date(year, month - 1, 1));
  }
  function next() {
    cursor = range === 'week' ? addDays(cursor, 7) : ymd(new Date(year, month + 1, 1));
  }
  function today() {
    cursor = todayIso;
  }
</script>

<div class="calendar">
  <header class="cal-head">
    <button class="nav" onclick={prev} aria-label="previous">‹</button>
    <span class="label">{label}</span>
    <button class="nav" onclick={next} aria-label="next">›</button>
    <button class="jump" onclick={today}>Today</button>
  </header>
  <div class="grid" class:week={range === 'week'}>
    {#each WEEKDAYS as wd (wd)}<span class="weekday">{wd}</span>{/each}
    {#each weeks as week, w (w)}
      {#each week as day (day.date)}
        <div class="cell" class:dim={!day.inMonth} class:is-today={day.date === todayIso}>
          <span class="num">{dayOfMonth(day.date)}</span>
          {#each byDay.get(day.date) ?? [] as card (card.id)}
            <button
              class="event"
              data-urgency={urgency(card.due)}
              onclick={() => onopen(card.id)}
              title={card.title ?? card.preview ?? ''}
            >
              {#if card.hard}<span class="hard" aria-hidden="true">◆</span>{/if}
              <span class="ev-title">{card.title ?? card.preview ?? card.id}</span>
            </button>
          {/each}
        </div>
      {/each}
    {/each}
  </div>
</div>

<style>
  .calendar {
    height: 100%;
    display: flex;
    flex-direction: column;
    padding: 1rem;
    box-sizing: border-box;
    gap: 0.6rem;
  }
  .cal-head {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }
  .label {
    font-size: 0.95rem;
    color: var(--text);
    min-width: 11rem;
  }
  .nav,
  .jump {
    background: var(--card-bg);
    border: 1px solid var(--card-border);
    border-radius: 7px;
    color: var(--text);
    cursor: pointer;
    font-size: 0.85rem;
    padding: 0.2rem 0.6rem;
  }
  .nav:hover,
  .jump:hover {
    border-color: var(--accent);
  }
  .jump {
    margin-left: auto;
  }
  .grid {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    grid-auto-rows: minmax(4.5rem, 1fr);
    gap: 1px;
    background: var(--column-border);
    border: 1px solid var(--column-border);
    border-radius: 8px;
    overflow: hidden;
  }
  /* One tall row for the week view. */
  .grid.week {
    grid-auto-rows: minmax(0, 1fr);
  }
  .weekday {
    background: var(--column-bg);
    color: var(--muted);
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 0.3rem 0.4rem;
    text-align: center;
  }
  .cell {
    background: var(--panel);
    padding: 0.25rem;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    overflow-y: auto;
    min-height: 0;
  }
  .cell.dim {
    background: var(--column-bg);
  }
  .cell.dim .num {
    color: var(--muted);
    opacity: 0.5;
  }
  .num {
    font-size: 0.72rem;
    color: var(--muted);
    align-self: flex-end;
  }
  .cell.is-today .num {
    color: var(--bg);
    background: var(--accent);
    border-radius: 999px;
    width: 1.2rem;
    height: 1.2rem;
    display: grid;
    place-items: center;
    font-weight: 700;
  }
  .event {
    display: flex;
    align-items: center;
    gap: 0.2rem;
    text-align: left;
    font: inherit;
    cursor: pointer;
    background: var(--card-bg);
    border: 1px solid var(--card-border);
    border-left: 3px solid var(--urgency, var(--muted));
    border-radius: 5px;
    padding: 0.12rem 0.3rem;
    color: var(--text);
    width: 100%;
  }
  .event:hover {
    border-color: var(--accent);
  }
  .ev-title {
    font-size: 0.72rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hard {
    color: var(--due-hard-fg);
    font-size: 0.62rem;
    flex-shrink: 0;
  }
</style>
