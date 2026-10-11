<script lang="ts">
  // What a `totals` view is mounted in: it holds the chosen day, asks for that day's numbers, and
  // opens whichever add sheet the view's group names. The renderers under it stay pure.
  //
  // It asks for its own data — like a note pane, and unlike the board — because a view's feed is
  // keyed by its name and a day is not part of a name. The feed it is handed is used as a signal:
  // when the app refreshes (an undo, another device's change), this asks again.
  import { untrack } from 'svelte';
  import Totals from '../renderers/Totals.svelte';
  import Trends from '../renderers/Trends.svelte';
  import SessionScreen from './training/SessionScreen.svelte';
  import * as nav from './nav.svelte';
  import { logDays, logRecent } from './ipc';
  import { createNotesUndoable, setPropertiesUndoable, undo } from './undo.svelte';
  import { addDays, today as todayNow } from './units';
  import type { Feed, Pane } from './panes';
  import type { DayTotals, NoteDraft, ObjectMeta, RecentItem, TotalsDef, ViewGroup } from './types';

  interface Props {
    pane: Pane;
    feed: Feed | undefined;
    /** Counts up each time the group's add control is pressed outside this pane. */
    addSignal?: number;
    /** Draw the group's views as a control in the pane — wherever the phone's group bar is not. */
    tabs?: boolean;
    onchange: (patch: Partial<Pane>) => void;
    onsaved: () => void;
  }
  let { pane, feed, addSignal = 0, tabs = false, onchange, onsaved }: Props = $props();

  /// **What is on screen: a view's name with its own definition, changed together.** Stepping to
  /// another of the group's views leaves the feed empty for a moment; reading the definition
  /// straight from it would blank the screen between every two tabs, and would for an instant ask
  /// for the new view's days with the old view's definition.
  let held = $state<{ view: string; def: TotalsDef; group: ViewGroup | undefined } | null>(null);
  $effect(() => {
    const t = feed?.view?.totals;
    if (t && pane.viewName) held = { view: pane.viewName, def: t, group: feed?.view?.group };
  });
  const def = $derived(held?.def);
  const group = $derived(held?.group ?? feed?.view?.group);
  const view = $derived(held?.view ?? '');
  /// The tab to mark: where the person asked to go, even before it has arrived.
  const wanted = $derived(pane.viewName ?? view);

  const today = todayNow();
  let selected = $state(today);
  let span = $state(7);

  let day = $state<DayTotals | null>(null);
  let before = $state<DayTotals | null>(null);
  let strip = $state<DayTotals[]>([]);
  let range = $state<DayTotals[]>([]);
  let last = $state<RecentItem[]>([]);
  let failed = $state('');

  /// The strip is a fortnight ending today, or around the chosen day once that is further back.
  /// …and it runs a week past today, so a meal can be planned ahead.
  const AHEAD = 7;
  const stripEnd = $derived(
    selected >= addDays(today, -13 + AHEAD) ? addDays(today, AHEAD) : addDays(selected, 6),
  );

  let asked = 0;
  async function load() {
    if (!def || !view) return;
    const mine = ++asked;
    try {
      if (def.span === 'range') {
        const r = await logDays(view, addDays(today, -(span - 1)), today);
        if (mine === asked) range = r.days;
        return;
      }
      // **One ask for the strip, the day and the day before it.** Each ask is a pass over the
      // whole vault on the other side, so the screen makes one (`crates/fm-app/tests/perf.rs`).
      const [all, recent] = await Promise.all([
        logDays(view, addDays(stripEnd, -14), stripEnd, true),
        def.add === 'workout' ? logRecent(view, { limit: 50 }) : Promise.resolve([]),
      ]);
      if (mine !== asked) return; // a later ask has already answered
      strip = all.days.slice(1);
      before = all.days.find((d) => d.day === addDays(selected, -1)) ?? null;
      day = all.days.find((d) => d.day === selected) ?? null;
      last = recent;
      failed = '';
    } catch (e) {
      if (mine === asked) failed = String(e);
    }
  }
  $effect(() => {
    // Ask again whenever any of these change — the feed included, which is the app saying
    // "something was written".
    void feed;
    void view;
    void selected;
    void span;
    void load();
  });
  function pick(d: string) {
    if (d === selected) return;
    day = null; // the old day's entries must not sit under the new day's date
    selected = d;
  }

  const yesterday = $derived.by(() => {
    const counts: Record<string, number> = {};
    for (const r of before?.rows ?? []) {
      if (r.props.planned) continue;
      const k = String(r.props.slot ?? '');
      counts[k] = (counts[k] ?? 0) + 1;
    }
    return counts;
  });

  // ── the add sheet ──────────────────────────────────────────────────────────────────────────────
  let sheet = $state<{ slot: string | null; entry: ObjectMeta | null } | null>(null);
  const closeSheet = () => (sheet = null);
  $effect(() => {
    // A layer, so Back closes the sheet before it leaves the view (`nav.svelte.ts`).
    if (sheet) nav.openLayer('log-sheet', closeSheet);
    else nav.closeLayer('log-sheet');
  });
  $effect(() => () => nav.closeLayer('log-sheet'));

  let seen = untrack(() => addSignal);
  $effect(() => {
    if (addSignal === seen) return;
    seen = addSignal;
    if (def?.add === 'food') sheet = { slot: null, entry: null };
    else if (def?.add === 'body') measuring = true;
    else if (def?.add === 'workout') workout = { set: null };
  });

  // ── writing down a measurement ─────────────────────────────────────────────────────────────────
  let measuring = $state(false);
  $effect(() => {
    if (measuring) nav.openLayer('measure-sheet', () => (measuring = false));
    else nav.closeLayer('measure-sheet');
  });
  $effect(() => () => nav.closeLayer('measure-sheet'));

  // ── the targets form ───────────────────────────────────────────────────────────────────────────
  let targetsOpen = $state(false);
  $effect(() => {
    if (targetsOpen) nav.openLayer('targets-sheet', () => (targetsOpen = false));
    else nav.closeLayer('targets-sheet');
  });
  $effect(() => () => nav.closeLayer('targets-sheet'));

  // ── a line that says what just happened, and takes it back ─────────────────────────────────────
  let toast = $state<{ text: string } | null>(null);
  let toastTimer: ReturnType<typeof setTimeout> | null = null;
  function say(text: string) {
    toast = { text };
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = null), 5000);
  }
  async function takeBack() {
    toast = null;
    try {
      await undo();
    } catch (e) {
      failed = String(e);
    }
    await load();
    onsaved();
  }

  async function wrote(what?: { removed?: string }) {
    if (what?.removed) say(`Removed “${what.removed}”`);
    await load();
    onsaved();
  }

  async function copyYesterday(slot: string) {
    if (!def) return;
    const drafts: NoteDraft[] = (before?.rows ?? [])
      .filter((r) => String(r.props.slot ?? '') === slot && !r.props.planned)
      .map((r) => ({
        title: r.title ?? '',
        start: selected,
        props: {
          ...Object.fromEntries(
            Object.entries(r.props).filter(
              (e): e is [string, string | number] =>
                typeof e[1] === 'string' || typeof e[1] === 'number',
            ),
          ),
          // Copied onto a day still to come, it is a plan.
          ...(selected > today ? { planned: 1 } : {}),
        },
      }));
    if (!drafts.length) return;
    const label = def.slots.find((s) => s.value === slot)?.label ?? slot;
    try {
      await createNotesUndoable(drafts, `copied ${label} from the day before`);
      say(`Added ${drafts.length} to ${label}`);
    } catch (e) {
      failed = String(e);
    }
    await wrote();
  }

  /// A planned entry happened: clearing the mark is what puts it in the day's totals.
  async function confirm(row: ObjectMeta) {
    try {
      await setPropertiesUndoable(
        row.id,
        { planned: '' },
        `confirmed “${row.title ?? 'an entry'}”`,
      );
    } catch (e) {
      failed = String(e);
    }
    await wrote();
  }

  // ── training: add an exercise, change a set, do last time's session again ───────────────────────
  let workout = $state<{ set: ObjectMeta | null } | null>(null);
  $effect(() => {
    if (workout) nav.openLayer('workout-sheet', () => (workout = null));
    else nav.closeLayer('workout-sheet');
  });
  $effect(() => () => nav.closeLayer('workout-sheet'));

  /// The sets of the most recent day that has any, planned again for today.
  async function repeatLast() {
    if (!def) return;
    try {
      const back = await logDays(view, addDays(today, -60), addDays(today, -1), true);
      const lastDay = back.days.filter((d) => (d.rows ?? []).some((r) => !r.props.planned)).at(-1);
      const drafts: NoteDraft[] = (lastDay?.rows ?? [])
        .filter((r) => !r.props.planned)
        .map((r) => {
          const { work_g: _work, session: _session, ...keep } = r.props;
          const props = Object.fromEntries(
            Object.entries(keep).filter(
              (e): e is [string, string | number] =>
                typeof e[1] === 'string' || typeof e[1] === 'number',
            ),
          );
          return { title: r.title ?? '', start: today, props: { ...props, planned: 1 } };
        });
      if (!drafts.length) {
        say('No earlier session to go by');
        return;
      }
      await createNotesUndoable(drafts, 'planned a session from the last one');
    } catch (e) {
      failed = String(e);
    }
    await wrote();
  }

  async function finishSet(set: ObjectMeta, reps: number, load_g: number) {
    try {
      await setPropertiesUndoable(
        set.id,
        { planned: '', work_g: reps * load_g },
        `finished a set of ${set.title ?? 'an exercise'}`,
      );
    } catch (e) {
      failed = String(e);
    }
    await wrote();
  }

  /// A bar in the trend was chosen: go to that day, in the group's day view.
  function openDay(d: string) {
    // Only where the group's first view is this log's own day screen.
    if (def?.add !== 'food') return;
    selected = d;
    day = null;
    const first = group?.views[0];
    if (first && first !== view) onchange({ viewName: first });
  }
</script>

<div class="tracker" class:tabbed={tabs}>
  {#if group && group.views.length > 1}
    <div class="tabs" role="group" aria-label={group.name}>
      {#each group.views as v (v)}
        <button
          type="button"
          class:on={v === wanted}
          aria-current={v === wanted ? 'page' : undefined}
          onclick={() => onchange({ viewName: v })}>{v}</button
        >
      {/each}
    </div>
  {/if}

  {#if failed}<p class="failed" role="alert">{failed}</p>{/if}

  <div class="screen">
    {#if !def}
      <p class="wait">Loading…</p>
    {:else if def.span === 'range'}
      <Trends
        {def}
        days={range}
        {span}
        onspan={(n) => (span = n)}
        onday={openDay}
        onadd={def.add === 'body' ? () => (measuring = true) : undefined}
      />
    {:else if def.add === 'workout'}
      <SessionScreen
        {def}
        {day}
        {last}
        onfinish={finishSet}
        onadd={() => (workout = { set: null })}
        onedit={(set) => (workout = { set })}
        onrepeat={repeatLast}
      />
    {:else}
      <Totals
        {def}
        {day}
        {strip}
        {selected}
        {today}
        {yesterday}
        onday={pick}
        onadd={(slot) => (sheet = { slot, entry: null })}
        onedit={(entry) => (sheet = { slot: null, entry })}
        oncopy={copyYesterday}
        ontargets={() => (targetsOpen = true)}
        onconfirm={confirm}
      />
    {/if}
  </div>

  {#if toast}
    <div class="toast" role="status">
      <span>{toast.text}</span>
      <button type="button" onclick={takeBack}>Undo</button>
    </div>
  {/if}

  {#if workout && def}
    {#await import('./training/WorkoutSheet.svelte') then { default: WorkoutSheet }}
      <WorkoutSheet
        {def}
        day={today}
        set={workout.set}
        {last}
        have={day?.rows ?? []}
        onclose={() => (workout = null)}
        onwrote={wrote}
      />
    {/await}
  {/if}

  {#if measuring && def}
    {#await import('./MeasureSheet.svelte') then { default: MeasureSheet }}
      <MeasureSheet {def} day={today} onclose={() => (measuring = false)} onsaved={wrote} />
    {/await}
  {/if}

  {#if targetsOpen && def}
    {#await import('./TargetsSheet.svelte') then { default: TargetsSheet }}
      <TargetsSheet
        {def}
        name={group?.name ?? view}
        current={day?.targets ?? {}}
        {today}
        onclose={() => (targetsOpen = false)}
        onsaved={wrote}
      />
    {/await}
  {/if}

  {#if sheet && def}
    {#await import('./food/LogSheet.svelte') then { default: LogSheet }}
      <LogSheet
        {view}
        {def}
        day={selected}
        slot={sheet.slot}
        entry={sheet.entry}
        onclose={closeSheet}
        onwrote={wrote}
      />
    {/await}
  {/if}
</div>

<style>
  .tracker {
    position: relative;
    height: 100%;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .screen {
    flex: 1;
    min-height: 0;
  }
  /* Where the phone's group bar is not: a wide screen, or several windows side by side. */
  .tabs {
    display: none;
    align-self: center;
    margin-top: var(--space-2);
    padding: 2px;
    border-radius: var(--radius-pill);
    background: var(--surface-hover);
  }
  .tracker.tabbed .tabs {
    display: flex;
  }
  @media (min-width: 60rem) {
    .tabs {
      display: flex;
    }
  }
  .tabs button {
    min-height: 2.25rem;
    padding: 0 var(--space-4);
    border: 0;
    border-radius: var(--radius-pill);
    background: none;
    color: var(--text-muted);
    font: inherit;
    cursor: pointer;
  }
  .tabs button.on {
    background: var(--surface-elevated);
    color: var(--text);
    box-shadow: var(--shadow-sm);
  }
  .failed {
    margin: var(--space-2) var(--space-3) 0;
    font-size: var(--text-sm);
    color: var(--danger-fg);
  }
  .wait {
    padding: var(--space-4);
    color: var(--text-muted);
  }
  .toast {
    position: absolute;
    left: 50%;
    bottom: var(--space-4);
    transform: translateX(-50%);
    max-width: calc(100% - 2 * var(--space-3));
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-2) var(--space-2) var(--space-4);
    border-radius: var(--radius-pill);
    background: var(--text);
    color: var(--bg);
    box-shadow: var(--shadow-md);
    font-size: var(--text-sm);
    animation: toast-in var(--dur-med) var(--ease);
  }
  .toast button {
    min-height: 2.25rem;
    padding: 0 var(--space-3);
    border: 0;
    border-radius: var(--radius-pill);
    background: none;
    color: inherit;
    font: inherit;
    font-weight: 700;
    cursor: pointer;
  }
  @keyframes toast-in {
    from {
      opacity: 0;
      transform: translate(-50%, 0.5rem);
    }
  }
</style>
