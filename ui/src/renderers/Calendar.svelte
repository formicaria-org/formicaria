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
    isoDate,
    clampRangeToWeek,
    assignLanes,
    type Day,
  } from '../lib/calendar';
  import { urgency } from '../lib/urgency';
  import type { ObjectMeta } from '../lib/types';

  // A renderer over the same agenda cards (dated, open) the list view uses — a
  // month or week grid instead of a list. Each note is a *bar* from its creation
  // day (start) to its due day (end), like a multi-day calendar event; a bar that
  // crosses a week boundary is split into one flat-ended segment per week row.
  // `range` picks the granularity; a single cursor date drives both. Bars are
  // tinted by *derived* urgency, never a stored priority. No status literal here.
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

  interface BarSeg {
    card: ObjectMeta;
    startCol: number;
    endCol: number;
    continuesLeft: boolean;
    continuesRight: boolean;
  }

  // Every dated card that touches this week, packed into non-overlapping lanes.
  // Start = creation day, end = due day; a due that predates creation (shouldn't
  // happen, but guard) collapses to a single day so the bar never runs backwards.
  function barsForWeek(week: Day[]): (BarSeg & { lane: number })[] {
    const segs: BarSeg[] = [];
    for (const c of cards) {
      if (!c.due) continue;
      const end = c.due.slice(0, 10);
      let start = c.created ? isoDate(c.created) : end;
      if (start > end) start = end;
      const span = clampRangeToWeek(start, end, week);
      if (span) segs.push({ card: c, ...span });
    }
    return assignLanes(segs);
  }

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
    <div class="weekday-row">
      {#each WEEKDAYS as wd (wd)}<span class="weekday">{wd}</span>{/each}
    </div>
    {#each weeks as week, w (w)}
      <div class="week-row">
        <div class="day-nums">
          {#each week as day (day.date)}
            <span class="num-cell" class:dim={!day.inMonth} class:is-today={day.date === todayIso}>
              <span class="num">{dayOfMonth(day.date)}</span>
            </span>
          {/each}
        </div>
        <div class="bars">
          {#each barsForWeek(week) as bar (bar.card.id)}
            <button
              class="event"
              class:cont-left={bar.continuesLeft}
              class:cont-right={bar.continuesRight}
              data-urgency={urgency(bar.card.due)}
              style="grid-column: {bar.startCol + 1} / {bar.endCol + 2}; grid-row: {bar.lane + 1};"
              onclick={() => onopen(bar.card.id)}
              title={bar.card.title ?? bar.card.preview ?? ''}
            >
              {#if bar.card.hard}<span class="hard" aria-hidden="true">◆</span>{/if}
              <span class="ev-title">{bar.card.title ?? bar.card.preview ?? bar.card.id}</span>
            </button>
          {/each}
        </div>
      </div>
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
    display: flex;
    flex-direction: column;
    background: var(--panel);
    border: 1px solid var(--column-border);
    border-radius: 8px;
    overflow: hidden;
  }
  .weekday-row {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    background: var(--column-bg);
    border-bottom: 1px solid var(--column-border);
  }
  .weekday {
    color: var(--muted);
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 0.3rem 0.4rem;
    text-align: center;
  }
  .week-row {
    flex: 1;
    min-height: 4.5rem;
    display: flex;
    flex-direction: column;
    border-bottom: 1px solid var(--column-border);
    overflow: hidden;
  }
  .week-row:last-child {
    border-bottom: none;
  }
  .day-nums {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
  }
  .num-cell {
    padding: 0.2rem 0.35rem;
    text-align: right;
    border-right: 1px solid var(--column-border);
  }
  .num-cell:last-child {
    border-right: none;
  }
  .num {
    font-size: 0.72rem;
    color: var(--muted);
  }
  .num-cell.dim .num {
    opacity: 0.45;
  }
  .num-cell.is-today .num {
    color: var(--bg);
    background: var(--accent);
    border-radius: 999px;
    padding: 0.05rem 0.35rem;
    font-weight: 700;
  }
  /* Bars share the same 7 columns as the day numbers, so a span lines up under
     the days it covers. Column guides run the full height behind the bars. */
  .bars {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    grid-auto-rows: 1.4rem;
    row-gap: 2px;
    padding-bottom: 0.25rem;
    overflow-y: auto;
    background: repeating-linear-gradient(
      to right,
      transparent 0,
      transparent calc(100% / 7 - 1px),
      var(--column-border) calc(100% / 7 - 1px),
      var(--column-border) calc(100% / 7)
    );
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
    padding: 0.05rem 0.35rem;
    margin: 0 3px;
    color: var(--text);
    min-width: 0;
    overflow: hidden;
  }
  .event:hover {
    border-color: var(--accent);
  }
  /* Flatten the edge where the bar runs off into an adjacent week. */
  .event.cont-left {
    border-top-left-radius: 0;
    border-bottom-left-radius: 0;
    border-left-width: 0;
    margin-left: 0;
  }
  .event.cont-right {
    border-top-right-radius: 0;
    border-bottom-right-radius: 0;
    margin-right: 0;
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
