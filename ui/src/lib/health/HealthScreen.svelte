<script lang="ts">
  // **The Health diary** (`decisions.md#ui` `#data`, 2026-10-22, *a fifth group, Health*): the day,
  // with the same rows every day whether or not anything happened (IHS, Tassorelli 2018), and the
  // month, one tracker at a time with its counts. It records and counts; it never labels a day.
  //
  // A row left untouched stays blank, which is *not recorded*. A recorded 0 is data. Touching any
  // control makes the day's note, so there is no separate "new entry" step.
  import { untrack } from 'svelte';
  import * as nav from '../nav.svelte';
  import { healthDays } from '../ipc';
  import { createNotesUndoable, setPropertiesUndoable } from '../undo.svelte';
  import { addDays, dayLabel } from '../units';
  import { formatCivil, weekday } from '../civil';
  import type { HealthDays, HealthWeek, TotalsDef, Tracker } from '../types';
  import Bars from '../charts/Bars.svelte';
  import ManualLink from '../ManualLink.svelte';
  import DoctorPage from './DoctorPage.svelte';
  import GuidanceScreen from './GuidanceScreen.svelte';
  import TrackSheet from './TrackSheet.svelte';
  import {
    BOTHER,
    BOTHER_CREDIT,
    SCORE_HOLDERS,
    SEVERITY,
    UNITS,
    WORDS,
    named,
    perOf,
    sectioned,
    sectionOf,
    CONTACTS_LINE,
    MIND_LINE,
  } from './starters';

  interface Props {
    def: TotalsDef;
    today: string;
    /** The app's feed, used as a signal: when it changes, ask again. */
    signal: unknown;
    /** Counts up each time the group's plus is pressed. */
    addSignal?: number;
    /** The day to open on: one chosen in another of the group's views, or today. */
    start?: string;
    /** A day was chosen in the month: open it in Diary. Back returns to the month. */
    onday?: (day: string) => void;
    onsaved: () => void;
    /** The window's id, so two Health windows keep their sheets and Back apart. */
    pane?: string;
  }
  let {
    def,
    today,
    signal,
    addSignal = 0,
    start,
    onday,
    onsaved,
    pane = 'health',
  }: Props = $props();

  let selected = $state(untrack(() => (start && start <= today ? start : today)));
  let month = $state(untrack(() => today.slice(0, 8) + '01'));
  let data = $state<HealthDays | null>(null);
  let failed = $state('');
  let picked = $state('');
  let tracking = $state(false);
  let doctor = $state(false);
  /** Over time, as a month's grid or as a bar a week: one tab, two ways of looking. */
  let shape = $state<'month' | 'weeks'>('month');
  /** Over time: how many weeks back, the week picked, and the yes/no tracker marked on it. */
  let weeksBack = $state(12);
  let pickedWeek = $state('');
  let mark = $state('');

  const endOf = (first: string) => addDays(addDays(first, 32).slice(0, 8) + '01', -1);
  const mondayOf = (d: string) => addDays(d, -((weekday(d) + 6) % 7));
  const monthly = $derived(def.span === 'range' && shape === 'month');
  const span = $derived(
    monthly
      ? [month, endOf(month)]
      : def.span === 'range'
        ? [addDays(mondayOf(today), -7 * (weeksBack - 1)), today]
        : [selected, selected],
  );

  async function load() {
    // In Guidance too: the plus there tracks something new, and must know the person's template.
    try {
      data = await healthDays(span[0], span[1], def.span === 'range' && !monthly);
      failed = '';
    } catch (e) {
      failed = String(e);
    }
  }
  $effect(() => {
    void signal;
    void span;
    untrack(load);
  });

  // The plus means what this view makes: something new to track.
  let seen = untrack(() => addSignal);
  $effect(() => {
    if (addSignal === seen) return;
    seen = addSignal;
    tracking = true;
  });
  $effect(() => {
    if (tracking) nav.openLayer(`health-track:${pane}`, () => (tracking = false));
    else nav.closeLayer(`health-track:${pane}`);
  });
  $effect(() => () => nav.closeLayer(`health-track:${pane}`));
  $effect(() => {
    if (doctor) nav.openLayer(`health-doctor:${pane}`, () => (doctor = false));
    else nav.closeLayer(`health-doctor:${pane}`);
  });
  $effect(() => () => nav.closeLayer(`health-doctor:${pane}`));

  // Each with its plain name where the key's own words read badly (`starters.ts` `LABELS`).
  const trackers = $derived((data?.trackers ?? []).map(named));
  /** The things tracked, in the Diary's sections (`starters.ts`). */
  const sections = $derived(sectioned(trackers));
  const day = $derived(data?.days.find((d) => d.day === selected));
  $effect(() => {
    // The first in the Diary's own order, so Over time opens where the Diary does.
    if (trackers.length && !trackers.some((t) => t.key === picked))
      picked = sections[0]?.items[0]?.key ?? trackers[0].key;
  });

  // ── writing a day ──────────────────────────────────────────────────────────────────────────
  /** Writes one after another, so two quick taps on a day with no note yet make one note. */
  let queue: Promise<void> = Promise.resolve();
  function record(t: Tracker, value: number | null) {
    const at = selected;
    // Shown at once; the answer from the server replaces it.
    if (data && day) {
      const values = { ...day.values };
      if (value === null) delete values[t.key];
      else values[t.key] = value;
      data = { ...data, days: data.days.map((d) => (d.day === at ? { ...d, values } : d)) };
    }
    const label =
      value === null ? `cleared ${t.label.toLowerCase()}` : `noted ${t.label.toLowerCase()}`;
    queue = queue
      .then(async () => {
        const id = data?.days.find((d) => d.day === at)?.id;
        if (id) await setPropertiesUndoable(id, { [t.key]: value ?? '' }, label);
        else if (value !== null) {
          const [made] = await createNotesUndoable(
            [{ title: 'Health diary', start: at, props: { log: def.log, [t.key]: value } }],
            label,
          );
          if (data)
            data = {
              ...data,
              days: data.days.map((d) => (d.day === at ? { ...d, id: made.id } : d)),
            };
        }
        onsaved();
      })
      .catch((e) => {
        failed = String(e);
        void load();
      });
  }

  const valueOf = (t: Tracker) => day?.values[t.key];
  const unitLabel = (u: string) => UNITS.find((x) => x.unit === u)?.label ?? u;
  const holder = (key: string) => SCORE_HOLDERS[key.replace(/_score$/, '')];
  const bothers = $derived(trackers.some((t) => t.kind === 'bother'));

  /** The cycle's context (`cycle.rs`): words on the day it begins, `none` on the day it ends. */
  const cycling = $derived(trackers.some((t) => t.key === 'period_start_yes'));
  function setContext(raw: string) {
    const at = selected;
    const words = raw.trim();
    queue = queue
      .then(async () => {
        const id = data?.days.find((d) => d.day === at)?.id;
        if (id) await setPropertiesUndoable(id, { cycle_context: words }, 'set the cycle context');
        else if (words)
          await createNotesUndoable(
            [{ title: 'Health diary', start: at, props: { log: def.log, cycle_context: words } }],
            'set the cycle context',
          );
        onsaved();
        await load();
      })
      .catch((e) => {
        failed = String(e);
      });
  }

  /** Tenths are typed as a decimal and kept whole: 38.1 °C is kept as 381. */
  const per = (t: Tracker) => (t.kind === 'amount' ? perOf(t.unit) : 1);
  function typedNumber(t: Tracker, raw: string) {
    const s = raw.trim();
    if (s === '') return record(t, null);
    const n = Math.round(Number(s) * per(t));
    if (Number.isFinite(n) && n >= 0) record(t, n);
  }

  // ── what fm already knows: Food's log and the body log ─────────────────────────────────────
  /** Read, not typed again: each with its name, and its value in the unit people read it in. */
  const ELSEWHERE: { key: string; label: string; show: (v: number) => string }[] = [
    { key: 'water_mg', label: 'Water, from Food', show: (v) => `${Math.round(v / 1000)} ml` },
    { key: 'caffeine_ug', label: 'Caffeine, from Food', show: (v) => `${Math.round(v / 1000)} mg` },
    { key: 'alcohol_mg', label: 'Alcohol, from Food', show: (v) => `${(v / 1000).toFixed(1)} g` },
    { key: 'weight_g', label: 'Weight', show: (v) => `${(v / 1000).toFixed(1)} kg` },
    { key: 'steps', label: 'Steps', show: (v) => v.toLocaleString() },
  ];
  const known = $derived(ELSEWHERE.filter((e) => day?.elsewhere?.[e.key] !== undefined));

  // ── the month ──────────────────────────────────────────────────────────────────────────────
  const monthName = $derived(formatCivil(month, { month: 'long', year: 'numeric' }));
  /** The month's days in weeks from Monday; `null` pads the first and last week. */
  const weeks = $derived.by(() => {
    const days = data?.days ?? [];
    if (!monthly || days.length === 0) return [];
    const lead = (weekday(days[0].day) + 6) % 7;
    const cells: ((typeof days)[number] | null)[] = [...Array(lead).fill(null), ...days];
    while (cells.length % 7) cells.push(null);
    const out = [];
    for (let i = 0; i < cells.length; i += 7) out.push(cells.slice(i, i + 7));
    return out;
  });
  const pickedTracker = $derived(trackers.find((t) => t.key === picked));
  const count = $derived(data?.counts[picked]);
  /** A cell's shade: the value against the kind's top, in four steps; nothing when not recorded. */
  function level(t: Tracker, v: number | undefined): string {
    if (v === undefined) return 'none';
    if (v === 0) return '0';
    const top =
      t.kind === 'severity' ? 3 : t.kind === 'rating' ? 10 : t.kind === 'bother' ? 100 : 0;
    if (!top) return '3';
    return String(Math.min(3, Math.max(1, Math.ceil((v / top) * 3))));
  }
  const daysOf = (n: number) => (n === 1 ? '1 day' : `${n} days`);
  const shown = (t: Tracker, v: number) =>
    t.kind === 'severity'
      ? (WORDS[t.key] ?? SEVERITY)[v]
      : t.kind === 'yesno'
        ? v
          ? 'Yes'
          : 'No'
        : per(t) > 1
          ? (v / per(t)).toFixed(1)
          : String(v);

  // ── over time ──────────────────────────────────────────────────────────────────────────────
  // **A week is the unit**, as in Fitness (`decisions.md#ui`, 2026-10-16): how many days of it, or
  // how many times, or the average of the days recorded — and always how many days were recorded,
  // so a week of two recorded days is not read as a week.
  const WINDOWS = [
    { weeks: 4, label: '4 weeks' },
    { weeks: 12, label: '12 weeks' },
    { weeks: 26, label: '6 months' },
    { weeks: 52, label: 'A year' },
  ];
  const isDays = (t: Tracker) => t.kind === 'severity' || t.kind === 'yesno';
  /** What a week's bar is, for this kind; `null` when nothing that week was recorded. */
  function weekValue(t: Tracker, w: HealthWeek): number | null {
    if (isDays(t)) return w.recorded ? w.days : null;
    if (t.kind === 'count') return w.recorded ? w.sum : null;
    return w.recorded ? Math.round((w.sum / w.recorded) * 10) / 10 : null;
  }
  function measure(t: Tracker): string {
    if (t.kind === 'severity') return 'Days in the week at mild or worse';
    if (t.kind === 'yesno') return 'Days in the week answered yes';
    if (t.kind === 'count') return 'Times in the week';
    if (t.kind === 'amount') return `Average of the days recorded, ${unitLabel(t.unit)}`;
    return 'Average of the days recorded';
  }
  const trackWeeks = $derived(data?.weeks?.[picked] ?? []);
  const markable = $derived(trackers.filter((t) => t.kind === 'yesno' && t.key !== picked));
  const markWeeks = $derived(mark ? (data?.weeks?.[mark] ?? []) : []);
  const tick = (w: string) => formatCivil(w, { day: 'numeric', month: 'numeric' });
  const bars = $derived(
    pickedTracker
      ? trackWeeks.map((w, i) => {
          const v = weekValue(pickedTracker, w);
          // A label every week fits only a few weeks wide; a phone has room for six.
          const every = trackWeeks.length > 13 ? 4 : trackWeeks.length > 6 ? 2 : 1;
          return {
            key: w.week,
            value: v ?? 0,
            label: `Week of ${dayLabel(w.week)}: ${v === null ? 'nothing recorded' : v}, recorded on ${w.recorded} of ${daysOf(w.of)}`,
            tick: i % every === 0 ? tick(w.week) : '',
          };
        })
      : [],
  );
  const weekShown = $derived(
    trackWeeks.find((w) => w.week === pickedWeek) ?? trackWeeks[trackWeeks.length - 1],
  );
  const markShown = $derived(markWeeks.find((w) => w.week === weekShown?.week));
  const markLabel = $derived(trackers.find((t) => t.key === mark)?.label ?? '');
</script>

{#if def.span === 'plan'}
  <GuidanceScreen {today} {signal} {onsaved} {pane} />
{:else}
  <div class="health">
    {#if failed}<p class="failed" role="alert">{failed}</p>{/if}

    {#if def.span === 'range'}
      {#if trackers.length === 0}
        <p class="empty">Nothing tracked yet. Start in Diary, or press ＋.</p>
      {:else}
        <div class="seg" role="group" aria-label="as">
          <button
            type="button"
            class:on={shape === 'month'}
            aria-pressed={shape === 'month'}
            onclick={() => (shape = 'month')}>Month</button
          >
          <button
            type="button"
            class:on={shape === 'weeks'}
            aria-pressed={shape === 'weeks'}
            onclick={() => (shape = 'weeks')}>Weeks</button
          >
        </div>
        <label class="pick">
          <span>Showing</span>
          <select bind:value={picked}>
            {#each sections as sec (sec.name)}
              <optgroup label={sec.name}>
                {#each sec.items as t (t.key)}<option value={t.key}>{t.label}</option>{/each}
              </optgroup>
            {/each}
          </select>
        </label>
        {#if shape === 'month'}
          <header class="step">
            <button
              type="button"
              aria-label="month before"
              onclick={() => (month = addDays(month, -1).slice(0, 8) + '01')}>‹</button
            >
            <h2>{monthName}</h2>
            <button
              type="button"
              aria-label="month after"
              disabled={month >= today.slice(0, 8) + '01'}
              onclick={() => (month = addDays(endOf(month), 1))}>›</button
            >
          </header>
          {#if pickedTracker}
            <div class="grid" aria-label={`${pickedTracker.label}, ${monthName}`}>
              {#each ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'] as w (w)}<span class="wd"
                  >{w}</span
                >{/each}
              {#each weeks as week, i (i)}
                {#each week as cell, j (j)}
                  {#if cell}
                    {@const v = cell.values[picked]}
                    <!-- A day is opened in Diary, to see it whole or fill it in. -->
                    <button
                      type="button"
                      class="cell"
                      data-level={level(pickedTracker, v)}
                      data-late={cell.late ? '1' : undefined}
                      disabled={cell.day > today || !onday}
                      aria-label={`${dayLabel(cell.day)}: ${v === undefined ? 'not recorded' : shown(pickedTracker, v)}${cell.late ? ', written in later' : ''}`}
                      onclick={() => onday?.(cell.day)}
                    >
                      <span class="n">{Number(cell.day.slice(8))}</span>
                      {#if v !== undefined && pickedTracker.kind !== 'severity' && pickedTracker.kind !== 'yesno'}<span
                          class="v">{v}</span
                        >{/if}
                    </button>
                  {:else}
                    <span class="pad"></span>
                  {/if}
                {/each}
              {/each}
            </div>
            {#if count}
              <dl class="counts">
                <dt>Recorded</dt>
                <dd>{count.recorded} of {daysOf(count.of)}</dd>
                <dt>{pickedTracker.kind === 'yesno' ? 'Yes' : 'At 1 or more'}</dt>
                <dd>{daysOf(count.days)}</dd>
                {#if count.moderate_or_severe !== undefined}
                  <dt>
                    {WORDS[picked]
                      ? `${WORDS[picked][2]} or ${WORDS[picked][3]}`
                      : 'Moderate or severe'}
                  </dt>
                  <dd>{daysOf(count.moderate_or_severe)}</dd>
                {/if}
              </dl>
              {#if count.odd}<p class="note">
                  {count.odd === 1 ? '1 value' : `${count.odd} values`} could not be read and {count.odd ===
                  1
                    ? 'is'
                    : 'are'} not counted.
                </p>{/if}
              {#if data?.days.some((d) => d.late && d.values[picked] !== undefined)}
                <p class="note"><span class="dot"></span> written in more than a day later</p>
              {/if}
            {/if}
          {/if}
        {:else}
          <div class="seg" role="group" aria-label="how far back">
            {#each WINDOWS as w (w.weeks)}
              <button
                type="button"
                class:on={weeksBack === w.weeks}
                aria-pressed={weeksBack === w.weeks}
                onclick={() => (weeksBack = w.weeks)}>{w.label}</button
              >
            {/each}
          </div>
          {#if pickedTracker}
            <p class="measure">{measure(pickedTracker)}</p>
            <Bars
              {bars}
              most={isDays(pickedTracker) ? 7 : 0}
              selected={weekShown?.week}
              onpick={(k) => {
                pickedWeek = k;
              }}
            />
            {#if mark && markWeeks.length}
              <div class="marks" aria-label={`${markLabel}, days a week`}>
                {#each markWeeks as w (w.week)}
                  <span title={`${markLabel}: ${daysOf(w.days)}`}
                    >{w.days ? '•'.repeat(Math.min(w.days, 3)) : ''}</span
                  >
                {/each}
              </div>
            {/if}
            {#if weekShown}
              {@const v = weekValue(pickedTracker, weekShown)}
              <p class="week">
                Week of {dayLabel(weekShown.week)}:
                <strong>{v === null ? 'nothing recorded' : v}</strong>, recorded on {weekShown.recorded}
                of
                {daysOf(weekShown.of)}{markShown
                  ? `; ${markLabel.toLowerCase()} on ${daysOf(markShown.days)}`
                  : ''}.
              </p>
            {/if}
            {#if markable.length}
              <label class="pick">
                <span>Mark</span>
                <select bind:value={mark}>
                  <option value="">Nothing</option>
                  {#each markable as t (t.key)}<option value={t.key}>{t.label}</option>{/each}
                </select>
              </label>
            {/if}
          {/if}
        {/if}
        {#if pickedTracker && sectionOf(pickedTracker.key) === 'Mood and activity'}
          <p class="mind" role="note">{CONTACTS_LINE}</p>
        {/if}
        <button type="button" class="go quiet" onclick={() => (doctor = true)}
          >For the doctor</button
        >
      {/if}
    {:else}
      <header class="step">
        <button
          type="button"
          aria-label="day before"
          onclick={() => (selected = addDays(selected, -1))}>‹</button
        >
        <h2>{selected === today ? 'Today' : dayLabel(selected)}</h2>
        <button
          type="button"
          aria-label="day after"
          disabled={selected >= today}
          onclick={() => (selected = addDays(selected, 1))}>›</button
        >
      </header>
      {#if day?.late}<p class="note">Written in more than a day later.</p>{/if}

      {#if data && trackers.length === 0}
        <div class="empty">
          <p>Nothing tracked yet.</p>
          <button type="button" class="go" onclick={() => (tracking = true)}>Track something</button
          >
        </div>
      {/if}

      {#each sections as sec (sec.name)}
        {#if sections.length > 1}<h3 class="section">{sec.name}</h3>{/if}
        <ul class="rows" aria-label={sec.name}>
          {#each sec.items as t (t.key)}
            {@const v = valueOf(t)}
            <li>
              <span class="label">{t.label}</span>
              {#if t.kind === 'severity'}
                <div class="chips" role="group" aria-label={t.label}>
                  {#each WORDS[t.key] ?? SEVERITY as word, n (n)}
                    <button
                      type="button"
                      class:on={v === n}
                      aria-pressed={v === n}
                      onclick={() => record(t, v === n ? null : n)}>{word}</button
                    >
                  {/each}
                </div>
              {:else if t.kind === 'yesno'}
                <div class="chips" role="group" aria-label={t.label}>
                  <button
                    type="button"
                    class:on={v === 1}
                    aria-pressed={v === 1}
                    onclick={() => record(t, v === 1 ? null : 1)}>Yes</button
                  >
                  <button
                    type="button"
                    class:on={v === 0}
                    aria-pressed={v === 0}
                    onclick={() => record(t, v === 0 ? null : 0)}>No</button
                  >
                </div>
              {:else if t.kind === 'rating'}
                <div class="chips wrap" role="group" aria-label={t.label}>
                  {#each Array(11) as _, n (n)}
                    <button
                      type="button"
                      class:on={v === n}
                      aria-pressed={v === n}
                      onclick={() => record(t, v === n ? null : n)}>{n}</button
                    >
                  {/each}
                </div>
              {:else if t.kind === 'bother'}
                <div class="bother">
                  <input
                    type="range"
                    min="0"
                    max="100"
                    value={v ?? 0}
                    class:unset={v === undefined}
                    aria-label={t.label}
                    onchange={(e) => record(t, Number((e.currentTarget as HTMLInputElement).value))}
                  />
                  <span class="ends"
                    ><span>{BOTHER.low}</span><span>{v ?? '—'}</span><span>{BOTHER.high}</span
                    ></span
                  >
                </div>
              {:else if t.kind === 'count'}
                <div class="stepper" role="group" aria-label={t.label}>
                  <button
                    type="button"
                    aria-label="one less"
                    disabled={v === 0}
                    onclick={() => record(t, Math.max(0, (v ?? 1) - 1))}>−</button
                  >
                  <span class="count" aria-live="polite">{v ?? '—'}</span>
                  <button
                    type="button"
                    aria-label="one more"
                    onclick={() => record(t, (v ?? 0) + 1)}>＋</button
                  >
                </div>
              {:else}
                <div class="amount">
                  <input
                    type="number"
                    inputmode="numeric"
                    min="0"
                    step={per(t) > 1 ? 0.1 : 1}
                    value={v === undefined ? '' : v / per(t)}
                    aria-label={t.label}
                    onchange={(e) => typedNumber(t, (e.currentTarget as HTMLInputElement).value)}
                  />
                  {#if t.kind === 'amount'}<span>{unitLabel(t.unit)}</span>{/if}
                  {#if t.kind === 'score' && holder(t.key)}
                    <a href={holder(t.key).url} target="_blank" rel="noopener noreferrer"
                      >{holder(t.key).name}</a
                    >
                  {/if}
                </div>
              {/if}
            </li>
          {/each}
        </ul>
      {/each}
      {#if sections.some((x) => x.name === 'Mood and activity')}
        <p class="mind" role="note">{MIND_LINE} {CONTACTS_LINE}</p>
      {/if}
      {#if known.length}
        <h3 class="section">From Food and your body log</h3>
        <ul class="rows elsewhere" aria-label="From Food and your body log">
          {#each known as e (e.key)}
            <li>
              <span class="label">{e.label}</span><span>{e.show(day!.elsewhere![e.key])}</span>
            </li>
          {/each}
        </ul>
        <p class="note">
          Logged in Food and Fitness, and changed there; shown here so nothing is typed twice.
        </p>
      {/if}
      {#if cycling}
        <label class="context">
          <span class="label">Cycle context</span>
          <input
            type="text"
            value={day?.context ?? ''}
            placeholder="Started the pill, pregnant, after a birth… or none to end it"
            onchange={(e) => setContext((e.currentTarget as HTMLInputElement).value)}
          />
          <span class="note"
            >Set on the day it begins and “none” on the day it ends. Cycles while one is in force
            are left out of the counts, as FIGO's are.</span
          >
        </label>
      {/if}
      <p class="manual"><ManualLink page="user/health.html" section="the-diary" /></p>
      {#if bothers}<p class="note">{BOTHER_CREDIT}</p>{/if}
      {#if trackers.some((t) => t.kind === 'severity')}
        <p class="note">
          None to severe: the International Headache Society's four steps (Diener 2019).
        </p>
      {/if}
    {/if}
  </div>
{/if}

{#if doctor}
  <DoctorPage {month} {today} onclose={() => (doctor = false)} />
{/if}

{#if tracking}
  <TrackSheet
    template={data?.template ?? null}
    known={trackers.map((t) => t.key)}
    onclose={() => (tracking = false)}
    onsaved={() => {
      void load();
      onsaved();
    }}
  />
{/if}

<style>
  .health {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4) var(--space-6);
    max-width: 40rem;
    margin: 0 auto;
    width: 100%;
    box-sizing: border-box;
  }
  button,
  input,
  select {
    font: inherit;
    color: inherit;
  }
  .step {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .step h2 {
    margin: 0;
    font-size: var(--text-lg);
  }
  .step button {
    width: 2.75rem;
    height: 2.75rem;
    border: 0;
    border-radius: var(--radius-pill);
    background: none;
    font-size: var(--text-xl);
    cursor: pointer;
  }
  .step button:disabled {
    opacity: 0.3;
    cursor: default;
  }
  /* The manual's link, drawn as a link: the component's own button carries no style. */
  .manual {
    margin: 0;
  }
  .manual :global(.manual-link) {
    padding: 0;
    border: 0;
    background: none;
    font: inherit;
    font-size: var(--text-sm);
    color: var(--accent);
    text-decoration: underline;
    cursor: pointer;
  }
  .section {
    margin: var(--space-3) 0 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .mind {
    margin: var(--space-2) 0 0;
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    background: var(--surface-hover);
    font-size: var(--text-sm);
  }
  .elsewhere li {
    flex-direction: row;
    justify-content: space-between;
  }
  .rows {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }
  .rows li {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3) 0;
    border-bottom: 1px solid var(--border);
  }
  .label {
    font-weight: 600;
  }
  .chips {
    display: flex;
    gap: var(--space-1);
  }
  .chips.wrap {
    flex-wrap: wrap;
  }
  .chips button {
    flex: 1 1 0;
    min-height: 2.75rem;
    min-width: 2.75rem;
    padding: 0 var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    cursor: pointer;
  }
  .chips.wrap button {
    flex: 0 0 auto;
  }
  .chips button.on {
    border-color: var(--accent);
    background: var(--accent-subtle);
    font-weight: 600;
  }
  /* A phone, or any narrow window: four words in one row, eleven numbers in two. */
  @media (max-width: 40rem) {
    .health {
      padding-left: var(--space-3);
      padding-right: var(--space-3);
    }
    .chips button {
      padding: 0 var(--space-1);
      font-size: var(--text-sm);
    }
    .chips.wrap button {
      min-width: calc((100% - 5 * var(--space-1)) / 6);
    }
  }
  .stepper {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
  .stepper button {
    width: 2.75rem;
    height: 2.75rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    background: var(--surface);
    cursor: pointer;
  }
  .stepper .count {
    min-width: 2ch;
    text-align: center;
    font-size: var(--text-lg);
  }
  .bother input {
    width: 100%;
  }
  .bother input.unset {
    opacity: 0.4;
  }
  .ends {
    display: flex;
    justify-content: space-between;
    font-size: var(--text-xs);
    color: var(--text-muted);
  }
  .amount {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex-wrap: wrap;
  }
  .amount input {
    width: 7rem;
    min-height: 2.75rem;
    padding: 0 var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
  }
  .amount a {
    font-size: var(--text-sm);
  }
  .context {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-3) 0;
  }
  .context input {
    min-height: 2.75rem;
    padding: 0 var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
  }
  .pick {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .pick select {
    flex: 1;
    min-height: 2.75rem;
    padding: 0 var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: var(--space-1);
  }
  .wd {
    text-align: center;
    font-size: var(--text-xs);
    color: var(--text-muted);
  }
  .cell {
    position: relative;
    aspect-ratio: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
  }
  .cell {
    padding: 0;
    font: inherit;
    color: inherit;
    cursor: pointer;
  }
  .cell:disabled {
    cursor: default;
  }
  .cell[data-level='none'] {
    border-style: dashed;
    color: var(--text-subtle);
  }
  .cell[data-level='1'] {
    background: color-mix(in srgb, var(--accent) 20%, var(--surface));
  }
  .cell[data-level='2'] {
    background: color-mix(in srgb, var(--accent) 45%, var(--surface));
  }
  .cell[data-level='3'] {
    background: color-mix(in srgb, var(--accent) 75%, var(--surface));
    color: var(--accent-contrast);
  }
  .cell[data-late]::after,
  .dot {
    content: '';
    display: inline-block;
    width: 0.4rem;
    height: 0.4rem;
    border-radius: 50%;
    background: var(--text-muted);
  }
  .cell[data-late]::after {
    position: absolute;
    top: 0.25rem;
    right: 0.25rem;
  }
  .cell .n {
    font-size: var(--text-sm);
  }
  .cell .v {
    font-size: var(--text-xs);
  }
  .counts {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: var(--space-1) var(--space-3);
    margin: 0;
  }
  .counts dt {
    color: var(--text-muted);
  }
  .counts dd {
    margin: 0;
    font-weight: 600;
  }
  .note,
  .empty p {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .empty {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    align-items: flex-start;
  }
  .seg {
    display: flex;
    gap: var(--space-1);
  }
  .seg button {
    flex: 1;
    min-height: 2.75rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    cursor: pointer;
  }
  .seg button.on {
    border-color: var(--accent);
    background: var(--accent-subtle);
    font-weight: 600;
  }
  .measure,
  .week {
    margin: 0;
    font-size: var(--text-sm);
  }
  .measure {
    color: var(--text-muted);
  }
  .marks {
    display: flex;
    gap: 2px;
    margin-top: calc(-1 * var(--space-2));
  }
  .marks span {
    flex: 1;
    text-align: center;
    font-size: var(--text-xs);
    color: var(--accent);
    line-height: 1;
  }
  .go.quiet {
    align-self: flex-start;
    background: var(--surface-hover);
    color: var(--text);
  }
  .go {
    min-height: 3rem;
    padding: 0 var(--space-4);
    border: 0;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--accent-contrast);
    font-weight: 600;
    cursor: pointer;
  }
  .failed {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--danger-fg);
  }
</style>
