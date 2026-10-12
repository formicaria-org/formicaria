<script lang="ts" module>
  /// A day chosen in one view of a group, for the view that is about to show it.
  const waiting = new Map<string, string>();
  const carried = {
    give: (pane: string, day: string) => void waiting.set(pane, day),
    take: (pane: string) => {
      const day = waiting.get(pane);
      waiting.delete(pane);
      return day;
    },
  };
</script>

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
  import {
    logDays,
    logKitchen,
    logRecent,
    logRecipes,
    logRoutines,
    planProgress,
    recipeLibrary,
  } from './ipc';
  import type { Dish } from './plan/measure';
  import { createNotesUndoable, setPropertiesUndoable, undo } from './undo.svelte';
  import { addDays, clockOf, dayLabel, nowClock, today as todayNow } from './units';
  import type { Feed, Pane } from './panes';
  import type {
    DayTotals,
    NoteDraft,
    ObjectMeta,
    RecentItem,
    PlanProgress,
    Routine,
    TotalsDef,
    ViewGroup,
  } from './types';

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
  /// **The chosen day outlives a step to another of the group's views.** The pane is built anew
  /// when its view changes, so a day chosen on one tab (a bar of the trend, a day of the week)
  /// was lost on the way to the tab that shows it — and the day screen opened on today. It is
  /// handed across here, once, by the pane's id.
  let selected = $state(untrack(() => carried.take(pane.id)) ?? today);
  let span = $state(7);

  let day = $state<DayTotals | null>(null);
  let before = $state<DayTotals | null>(null);
  let strip = $state<DayTotals[]>([]);
  let range = $state<DayTotals[]>([]);
  let last = $state<RecentItem[]>([]);
  let routines = $state<Routine[]>([]);
  /// The chosen day against the diet the person follows, when they follow one.
  let followed = $state<PlanProgress | null>(null);
  /// What could be had to close a gap: the person's recipes (a serving of each, and how much of
  /// each is in the kitchen) and the recipes carried in the app. Asked for only when a diet is
  /// followed and something is off, and never the reason a day fails to draw.
  let dishes = $state<Dish[]>([]);
  let dishesFor = '';
  $effect(() => {
    const off = followed?.checks.some((c) => c.per === 'day' && c.state === 'under');
    if (!off || !view || dishesFor === view) return;
    dishesFor = view;
    void Promise.all([logRecipes(view), recipeLibrary(), logKitchen()])
      .then(([own, lib, kitchen]) => {
        const names = new Set(kitchen.map((k) => k.title.trim().toLowerCase()));
        dishes = [
          ...own.map((r) => ({
            title: r.title,
            from: 'yours',
            ref: `recipe/${r.recipe}`,
            per_serving: Object.fromEntries(
              Object.entries(r.sums).map(([k, v]) => [
                k,
                Math.round(v.sum / Math.max(1, r.servings)),
              ]),
            ),
            have: {
              n: r.ingredients.filter((i) => names.has((i.title ?? '').trim().toLowerCase()))
                .length,
              of: r.ingredients.length,
            },
          })),
          ...lib.recipes.map((r) => ({
            title: r.title,
            from: 'NHLBI',
            ref: `nhlbi/${r.id}`,
            per_serving: r.per_serving,
          })),
        ];
      })
      .catch(() => (dishes = []));
  });
  let failed = $state('');

  /// The strip is a fortnight ending today, or around the chosen day once that is further back.
  /// …and it runs a week past today, so a meal can be planned ahead.
  const AHEAD = 7;
  const stripEnd = $derived(
    selected >= addDays(today, -13 + AHEAD) ? addDays(today, AHEAD) : addDays(selected, 6),
  );

  let asked = 0;
  async function load() {
    // The plan screen asks for its own week (`plan/PlanScreen.svelte`).
    if (!def || !view || def.span === 'plan' || def.span === 'menu' || def.span === 'week') return;
    const mine = ++asked;
    try {
      if (def.span === 'range') {
        const r = await logDays(view, addDays(today, -(span - 1)), today);
        if (mine === asked) range = r.days;
        return;
      }
      // **One ask for the strip, the day and the day before it.** Each ask is a pass over the
      // whole vault on the other side, so the screen makes one (`crates/fm-app/tests/perf.rs`).
      const [all, recent, kept, held] = await Promise.all([
        logDays(view, addDays(stripEnd, -14), stripEnd, true),
        def.add === 'workout' ? logRecent(view, { limit: 50 }) : Promise.resolve([]),
        def.add === 'workout' ? logRoutines(view) : Promise.resolve([]),
        // Never the reason a day fails to draw.
        def.add === 'food'
          ? planProgress('food', selected).catch(() => null)
          : Promise.resolve(null),
      ]);
      if (mine !== asked) return; // a later ask has already answered
      strip = all.days.slice(1);
      before = all.days.find((d) => d.day === addDays(selected, -1)) ?? null;
      day = all.days.find((d) => d.day === selected) ?? null;
      last = recent;
      routines = kept;
      followed = held;
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
  let sheet = $state<{
    slot: string | null;
    entry: ObjectMeta | null;
    /** A recipe to open the sheet on: `recipe/<word>` or `nhlbi/<id>`. */
    open?: string;
  } | null>(null);
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
    else if (def?.add === 'body') measuring = def;
    else if (def?.add === 'workout') workout = { set: null };
  });

  // ── writing down a measurement ─────────────────────────────────────────────────────────────────
  /// The values to write down, as a definition: the view's own, or one value kept in another log.
  let measuring = $state<TotalsDef | null>(null);
  $effect(() => {
    if (measuring) nav.openLayer('measure-sheet', () => (measuring = null));
    else nav.closeLayer('measure-sheet');
  });
  $effect(() => () => nav.closeLayer('measure-sheet'));

  // ── every nutrient of the day ──────────────────────────────────────────────────────────────────
  let nutrientsOpen = $state(false);
  $effect(() => {
    if (nutrientsOpen) nav.openLayer('nutrients-sheet', () => (nutrientsOpen = false));
    else nav.closeLayer('nutrients-sheet');
  });
  $effect(() => () => nav.closeLayer('nutrients-sheet'));

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

  /// Counts what was written from this pane's sheets, for a screen that asks for its own days.
  let wrotes = $state(0);
  async function wrote(what?: { removed?: string }) {
    wrotes += 1;
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
        // Confirmed today, it was had now — unless it was planned for a time, which it keeps.
        {
          planned: '',
          ...(selected === today && !clockOf(row.start) ? { start: `${today}T${nowClock()}` } : {}),
        },
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
        // Sets only: an activity is logged when it is done, and has nothing to tick.
        .filter((r) => !r.props.planned && r.props.activity === undefined)
        .map((r) => {
          const { work_g: _work, session: _session, ...keep } = r.props;
          const props = Object.fromEntries(
            Object.entries(keep).filter(
              (e): e is [string, string | number] =>
                typeof e[1] === 'string' || typeof e[1] === 'number',
            ),
          );
          return { title: r.title ?? '', start: selected, props: { ...props, planned: 1 } };
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

  /// A routine's sets, planned for a day: the one in front, unless the week screen names another.
  async function planRoutine(r: Routine, on: string = selected) {
    const drafts: NoteDraft[] = r.sets.map((s) => {
      const { routine: _routine, ...keep } = s.props;
      const props = Object.fromEntries(
        Object.entries(keep).filter(
          (e): e is [string, string | number] =>
            typeof e[1] === 'string' || typeof e[1] === 'number',
        ),
      );
      return { title: s.title ?? '', start: on, props: { ...props, planned: 1 } };
    });
    if (!drafts.length) {
      say(`“${r.title}” has no sets`);
      return;
    }
    try {
      await createNotesUndoable(drafts, `planned “${r.title}”`);
    } catch (e) {
      failed = String(e);
    }
    await wrote();
  }
  let routineSheet = $state<{ routine: Routine | null } | null>(null);
  $effect(() => {
    if (routineSheet) nav.openLayer('routine-sheet', () => (routineSheet = null));
    else nav.closeLayer('routine-sheet');
  });
  $effect(() => () => nav.closeLayer('routine-sheet'));

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
    const first = group?.views[0];
    if (!first || first === view) return;
    carried.give(pane.id, d);
    onchange({ viewName: first });
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
    {:else if def.span === 'plan'}
      {#await import('./plan/PlanScreen.svelte') then { default: PlanScreen }}
        <PlanScreen {def} {today} signal={feed} {onsaved} />
      {/await}
    {:else if def.span === 'menu'}
      {#await import('./food/MenuScreen.svelte') then { default: MenuScreen }}
        <MenuScreen
          {view}
          {def}
          {today}
          signal={[feed, wrotes]}
          onadd={(d, slot) => {
            selected = d;
            sheet = { slot, entry: null };
          }}
          onedit={(entry, d) => {
            selected = d;
            sheet = { slot: null, entry };
          }}
          {onsaved}
        />
      {/await}
    {:else if def.span === 'week'}
      {#await import('./training/WeekScreen.svelte') then { default: WeekScreen }}
        <WeekScreen
          {view}
          {today}
          signal={[feed, wrotes]}
          onday={(d) => {
            const first = group?.views[0];
            if (!first || first === view) return;
            carried.give(pane.id, d);
            onchange({ viewName: first });
          }}
          onplan={(r, d) => planRoutine(r, d)}
        />
      {/await}
    {:else if def.span === 'range'}
      <Trends
        {def}
        days={range}
        {span}
        onspan={(n) => (span = n)}
        onday={openDay}
        onadd={def.add === 'body' ? () => (measuring = def ?? null) : undefined}
        onmeasure={(m) =>
          def &&
          m.log &&
          (measuring = { ...def, log: m.log, metrics: [{ ...m, role: 'primary' }] })}
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
        {routines}
        today={selected}
        label={selected === today ? '' : dayLabel(selected)}
        ontoday={selected === today ? undefined : () => pick(today)}
        onplan={(r) => planRoutine(r)}
        onkeep={() => (routineSheet = { routine: null })}
        onroutine={(routine) => (routineSheet = { routine })}
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
        plan={followed}
        {dishes}
        onidea={(dish) => (sheet = { slot: null, entry: null, open: dish.ref })}
        onnutrients={def.add === 'food' ? () => (nutrientsOpen = true) : undefined}
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
        day={selected}
        set={workout.set}
        {last}
        have={day?.rows ?? []}
        onclose={() => (workout = null)}
        onwrote={wrote}
      />
    {/await}
  {/if}

  {#if routineSheet && def}
    {@const which = routineSheet}
    {#await import('./training/RoutineSheet.svelte') then { default: RoutineSheet }}
      <RoutineSheet
        {def}
        routine={which.routine}
        sets={(day?.rows ?? []).filter((r) => r.props.activity === undefined)}
        {routines}
        onclose={() => (routineSheet = null)}
        onwrote={wrote}
      />
    {/await}
  {/if}
  {#if measuring}
    {@const what = measuring}
    {#await import('./MeasureSheet.svelte') then { default: MeasureSheet }}
      <MeasureSheet def={what} day={today} onclose={() => (measuring = null)} onsaved={wrote} />
    {/await}
  {/if}

  {#if nutrientsOpen}
    {#await import('./food/NutrientsSheet.svelte') then { default: NutrientsSheet }}
      <NutrientsSheet {view} day={selected} onclose={() => (nutrientsOpen = false)} />
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
        open={sheet.open}
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
  /* The group's views, on every width: a phone's bar is the app's own
     (`decisions.md#ui`, 2026-10-12), so this is where one steps from Day to Trends. */
  .tabs {
    display: flex;
    align-self: center;
    margin-top: var(--space-2);
    padding: 2px;
    border-radius: var(--radius-pill);
    background: var(--surface-hover);
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
