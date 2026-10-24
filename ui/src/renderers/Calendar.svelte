<script lang="ts">
  import { calledOff } from '../lib/calledOff';
  import {
    monthGrid,
    weekOf,
    monthLabel,
    weekLabel,
    WEEKDAYS,
    monthStart,
    dayOfMonth,
    addDays,
    clampRangeToWeek,
    assignLanes,
    timedDay,
    timedBlocks,
    type Day,
  } from '../lib/calendar';
  import { appNowMinutes, appToday } from '../lib/appTime';
  import { urgency } from '../lib/urgency';
  import { dayOf, timeRange } from '../lib/stamp';
  import VaultBadge from '../lib/VaultBadge.svelte';
  import type { ObjectMeta } from '../lib/types';

  // A renderer over the same agenda cards (dated, open) the list view uses — a
  // month or week grid instead of a list. Each note is a *bar* from its creation
  // day (start) to its due day (end), like a multi-day calendar event; a bar that
  // crosses a week boundary is split into one flat-ended segment per week row.
  // `range` picks the granularity; a single cursor date drives both. Bars are
  // tinted by *derived* urgency, never a stored priority. No status literal here.
  //
  // **The week is a time grid** (the owner, 2026-10-11: *in the weekly view I should see the tasks
  // in their time slot, not just a list, on all devices*). A note with a clock time sits in its
  // day's column at that time, as tall as it is long; a note with only a date, or one that spans
  // days, stays a bar in the strip above the hours, where the month draws it too. The month is
  // unchanged: it has no room for hours.
  let {
    cards,
    onopen,
    range = 'month',
  }: { cards: ObjectMeta[]; onopen: (id: string) => void; range?: 'month' | 'week' } = $props();

  // Today and now on the app's clock (2026-10-24), refreshed each minute below.
  let todayIso = $state(appToday());
  let cursor = $state(appToday()); // month view ignores the day-of-month

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
  // The bar runs from the note's optional `start` to its `due`. With no start it
  // collapses to a single day on the due date (a plain deadline) — creation date
  // is never assumed as a start. A start after the due (shouldn't happen) is
  // clamped so the bar never runs backwards.
  function barsForWeek(week: Day[]): (BarSeg & { lane: number })[] {
    const segs: BarSeg[] = [];
    for (const c of cards) {
      if (!c.due) continue;
      // The grid is day-granular: narrow both ends to their calendar day. A time,
      // when present, shows in the bar's label rather than moving its geometry.
      const end = dayOf(c.due);
      let start = dayOf(c.start) || end;
      if (start > end) start = end;
      // In the week, a note placed by the clock is drawn once, in the hours below.
      if (range === 'week' && timedDay(c)) continue;
      const span = clampRangeToWeek(start, end, week);
      if (span) segs.push({ card: c, ...span });
    }
    return assignLanes(segs);
  }

  function prev() {
    cursor = range === 'week' ? addDays(cursor, -7) : monthStart(year, month - 1);
  }
  function next() {
    cursor = range === 'week' ? addDays(cursor, 7) : monthStart(year, month + 1);
  }
  function today() {
    cursor = todayIso;
  }

  const HOURS = Array.from({ length: 24 }, (_, h) => h);
  const hourLabel = (h: number) => `${String(h).padStart(2, '0')}:00`;
  /** Where the hours open: an hour before the first thing of the week, never later than 07:00, so
   *  an ordinary day shows from the morning and an early start is not above the fold. */
  let firstHour = $derived.by(() => {
    if (range !== 'week') return 7;
    const starts = weeks[0].flatMap((d) => timedBlocks(cards, d.date).map((b) => b.from));
    return Math.max(0, Math.min(7, Math.floor(Math.min(...starts, 24 * 60) / 60) - 1));
  });
  let nowMinutes = $state(appNowMinutes());
  $effect(() => {
    const id = setInterval(() => {
      nowMinutes = appNowMinutes();
      todayIso = appToday();
    }, 60_000);
    return () => clearInterval(id);
  });

  /** Bring the useful part into view when a week opens: the first hour that matters, and on a
   *  screen too narrow for seven columns, today's column (or the week's first). Runs again when
   *  the week changes, and never while someone is scrolling within one. */
  let scroller = $state<HTMLElement | null>(null);
  $effect(() => {
    const el = scroller;
    const week = range === 'week' ? weeks[0] : null;
    if (!el || !week) return;
    const hour = el.querySelector<HTMLElement>('.hour-row');
    if (hour) el.scrollTop = firstHour * hour.offsetHeight;
    const col = el.querySelector<HTMLElement>('.day-col');
    const at = Math.max(
      0,
      week.findIndex((d) => d.date === todayIso),
    );
    if (col) el.scrollLeft = at * col.offsetWidth;
  });
</script>

<div class="calendar">
  <header class="cal-head">
    <button class="nav" onclick={prev} aria-label="previous">‹</button>
    <span class="label">{label}</span>
    <button class="nav" onclick={next} aria-label="next">›</button>
    <button class="jump" onclick={today}>Today</button>
  </header>
  {#if range === 'week'}
    <!-- One scroller for both directions: the day headings stay at the top and the hours at the
         side while the grid moves under them, on a wide screen and on a phone alike. -->
    <div class="grid week" bind:this={scroller}>
      <div class="week-inner">
        <div class="week-head">
          <span class="gutter" aria-hidden="true"></span>
          <div class="days">
            <div class="weekday-row">
              {#each WEEKDAYS as wd (wd)}<span class="weekday">{wd}</span>{/each}
            </div>
            {#each weeks as week, w (w)}
              <div class="week-row">
                <div class="day-nums">
                  {#each week as day (day.date)}
                    <span
                      class="num-cell"
                      class:dim={!day.inMonth}
                      class:is-today={day.date === todayIso}
                    >
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
                      class:called-off={calledOff(bar.card)}
                      data-urgency={urgency(bar.card.due)}
                      style="grid-column: {bar.startCol + 1} / {bar.endCol +
                        2}; grid-row: {bar.lane + 1};"
                      onclick={() => onopen(bar.card.id)}
                      title={`${bar.card.title ?? bar.card.preview ?? ''}${calledOff(bar.card) ? ' (called off)' : ''}`}
                    >
                      {#if bar.card.hard}<span class="hard" aria-hidden="true">◆</span>{/if}
                      {#if !bar.continuesLeft}<VaultBadge vault={bar.card.vault} dot />{/if}
                      {#if !bar.continuesLeft && timeRange(bar.card.start, bar.card.due)}
                        <span class="ev-time">{timeRange(bar.card.start, bar.card.due)}</span>
                      {/if}
                      <span class="ev-title"
                        >{bar.card.title ?? bar.card.preview ?? bar.card.id}</span
                      >
                    </button>
                  {/each}
                </div>
              </div>
            {/each}
          </div>
        </div>
        <div class="hours">
          <div class="gutter">
            {#each HOURS as h (h)}
              <span class="hour-row"><span class="hour-label">{hourLabel(h)}</span></span>
            {/each}
          </div>
          <div class="days day-cols">
            {#each weeks[0] as day (day.date)}
              <div class="day-col" class:is-today={day.date === todayIso} data-day={day.date}>
                {#if day.date === todayIso}
                  <span class="now-line" style="top: calc({nowMinutes} / 60 * var(--hour))"></span>
                {/if}
                {#each timedBlocks(cards, day.date) as b (b.card.id)}
                  <button
                    class="block"
                    class:short={b.to - b.from < 60}
                    class:clash={b.clash}
                    data-urgency={urgency(b.card.due)}
                    style="top: calc({b.from} / 60 * var(--hour)); height: calc({b.to -
                      b.from} / 60 * var(--hour)); left: {(b.col / b.cols) * 100}%; width: {100 /
                      b.cols}%;"
                    onclick={() => onopen(b.card.id)}
                    title={(b.clash ? 'Clashes with another meeting — ' : '') +
                      (b.card.title ?? b.card.preview ?? '')}
                  >
                    <span class="block-time">
                      {#if b.card.hard}<span class="hard" aria-hidden="true">◆</span>{/if}
                      <VaultBadge vault={b.card.vault} dot />
                      {b.to - b.from < 60
                        ? timeRange(b.card.start, b.card.due).slice(0, 5)
                        : timeRange(b.card.start, b.card.due)}
                    </span>
                    {#if b.clash}<span class="clash-mark">⚠ clash</span>{/if}
                    <span class="block-title">{b.card.title ?? b.card.preview ?? b.card.id}</span>
                  </button>
                {/each}
              </div>
            {/each}
          </div>
        </div>
      </div>
    </div>
  {:else}
    <div class="grid">
      <div class="weekday-row">
        {#each WEEKDAYS as wd (wd)}<span class="weekday">{wd}</span>{/each}
      </div>
      {#each weeks as week, w (w)}
        <div class="week-row">
          <div class="day-nums">
            {#each week as day (day.date)}
              <span
                class="num-cell"
                class:dim={!day.inMonth}
                class:is-today={day.date === todayIso}
              >
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
                style="grid-column: {bar.startCol + 1} / {bar.endCol + 2}; grid-row: {bar.lane +
                  1};"
                onclick={() => onopen(bar.card.id)}
                title={bar.card.title ?? bar.card.preview ?? ''}
              >
                {#if bar.card.hard}<span class="hard" aria-hidden="true">◆</span>{/if}
                {#if !bar.continuesLeft}<VaultBadge vault={bar.card.vault} dot />{/if}
                {#if !bar.continuesLeft && timeRange(bar.card.start, bar.card.due)}
                  <span class="ev-time">{timeRange(bar.card.start, bar.card.due)}</span>
                {/if}
                <span class="ev-title">{bar.card.title ?? bar.card.preview ?? bar.card.id}</span>
              </button>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {/if}
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
  /* The clock leads the label, like every other calendar. Tabular figures keep
     the titles aligned down a column of bars; it never shrinks away. */
  .ev-time {
    flex: 0 0 auto;
    font-size: 0.68rem;
    font-variant-numeric: tabular-nums;
    opacity: 0.85;
  }
  /* ---- The week's time grid ---- */
  .grid.week {
    --hour: 2.75rem;
    --gutter: 2.9rem;
    /* Seven columns never get narrower than this; a screen that cannot hold them scrolls sideways
       instead of squeezing a title to nothing. */
    --col-min: 5.5rem;
    display: block;
    overflow: auto;
    overscroll-behavior: contain;
  }
  .week-inner {
    min-width: calc(var(--gutter) + 7 * var(--col-min));
  }
  .week-head,
  .hours {
    display: flex;
  }
  .week-head {
    position: sticky;
    top: 0;
    z-index: 3;
    background: var(--panel);
    border-bottom: 1px solid var(--column-border);
  }
  .days {
    flex: 1;
    min-width: 0;
  }
  .gutter {
    flex: 0 0 var(--gutter);
    position: sticky;
    left: 0;
    z-index: 2;
    background: var(--panel);
    border-right: 1px solid var(--column-border);
    box-sizing: border-box;
  }
  .week-head .gutter {
    z-index: 4;
  }
  /* The strip above the hours holds what has a date and no time. It is as tall as it needs to be,
     up to a few rows, and never takes the hours' room. */
  .grid.week .week-row {
    flex: none;
    min-height: 0;
    border-bottom: none;
  }
  .grid.week .bars {
    flex: none;
    min-height: 0.5rem;
    max-height: 6rem;
  }
  .hour-row {
    display: block;
    height: var(--hour);
    box-sizing: border-box;
    text-align: right;
    padding-right: 0.3rem;
  }
  .hour-label {
    position: relative;
    top: -0.5em;
    font-size: 0.66rem;
    font-variant-numeric: tabular-nums;
    color: var(--muted);
  }
  .hour-row:first-child .hour-label {
    visibility: hidden; /* 00:00 would sit half above the grid */
  }
  .day-cols {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
  }
  .day-col {
    position: relative;
    height: calc(24 * var(--hour));
    border-right: 1px solid var(--column-border);
    background: repeating-linear-gradient(
      to bottom,
      transparent 0,
      transparent calc(var(--hour) - 1px),
      var(--column-border) calc(var(--hour) - 1px),
      var(--column-border) var(--hour)
    );
  }
  .day-col:last-child {
    border-right: none;
  }
  .day-col.is-today {
    background-color: color-mix(in srgb, var(--accent) 6%, transparent);
  }
  .now-line {
    position: absolute;
    left: 0;
    right: 0;
    height: 0;
    border-top: 2px solid var(--accent);
    z-index: 1;
    pointer-events: none;
  }
  /* Two meetings at once: said on the block, not left to the width of two halves. */
  .block.clash {
    border-color: var(--danger-fg);
    border-left-color: var(--danger-fg);
  }
  .clash-mark {
    align-self: flex-start;
    padding: 0 0.25rem;
    border-radius: 3px;
    font-size: 0.7rem;
    font-weight: 600;
    background: var(--danger-bg);
    color: var(--danger-fg);
  }
  .block {
    position: absolute;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 0.05rem;
    text-align: left;
    font: inherit;
    cursor: pointer;
    background: var(--card-bg);
    border: 1px solid var(--card-border);
    border-left: 3px solid var(--urgency, var(--muted));
    border-radius: 5px;
    padding: 0.1rem 0.3rem;
    color: var(--text);
    overflow: hidden;
    z-index: 1;
  }
  .block:hover {
    border-color: var(--accent);
    z-index: 2;
  }
  /* Under an hour there is one line of room: the time and the title share it. */
  .block.short {
    flex-direction: row;
    align-items: center;
    gap: 0.3rem;
    padding-block: 0;
  }
  .block.short .block-title {
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .block-time {
    flex: 0 0 auto;
    display: flex;
    align-items: center;
    gap: 0.2rem;
    font-size: 0.66rem;
    font-variant-numeric: tabular-nums;
    opacity: 0.85;
    white-space: nowrap;
  }
  .block-title {
    font-size: 0.72rem;
    line-height: 1.15;
    overflow: hidden;
    overflow-wrap: anywhere;
  }
  .hard {
    color: var(--due-hard-fg);
    font-size: 0.62rem;
    flex-shrink: 0;
  }
  /* A meeting called off keeps its place, struck through (see `lib/calledOff.ts`). */
  .event.called-off {
    opacity: 0.55;
  }
  .event.called-off .ev-title,
  .event.called-off .ev-time {
    text-decoration: line-through;
  }
</style>
