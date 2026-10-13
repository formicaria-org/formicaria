<script lang="ts">
  // Planning an exercise for a session — so many sets of so many reps at a load — or changing one
  // set already there.
  //
  // One note per set (`decisions.md#data`, 2026-10-11), each planned until it is ticked. The name
  // of an exercise is whatever the person types; the ones used before are offered, with the
  // numbers they had last time already filled in.
  //
  // **Or an activity, by its minutes and how hard it felt** (`decisions.md#data`, 2026-10-12): a
  // swim, a walk, a yoga class. That is the entry a weekly dose is measured from — WHO counts
  // minutes at an intensity, and takes the intensity from effort on a scale of 0 to 10 — and it
  // is one note, done when it is written. An exercise may also say which region of the body it
  // trains, which is what a strength regime counts.
  import { untrack } from 'svelte';
  import Icon from '../Icon.svelte';
  import { createNotesUndoable, setPropertiesUndoable, deleteUndoable } from '../undo.svelte';
  import { whole } from '../units';
  import { planLibrary } from '../ipc';
  import { exercises as cardLibrary, sameName } from './exercises';
  import Resumed from '../Resumed.svelte';
  import { draftOf, dropDraft, keepDraft } from '../drafts';
  import type {
    ExerciseCard,
    NoteDraft,
    ObjectMeta,
    PlanActivity,
    RecentItem,
    TotalsDef,
  } from '../types';

  interface Props {
    def: TotalsDef;
    /** An exercise chosen from its card, to start the form from. */
    preset?: { name: string; region: string };
    day: string;
    /** A set to change, instead of planning an exercise. */
    set?: ObjectMeta | null;
    /** What each exercise was last time. */
    last: RecentItem[];
    /** The day's sets so far, so a new one is numbered after them. */
    have: ObjectMeta[];
    onclose: () => void;
    onwrote: (what?: { removed?: string }) => void;
  }
  let { def, day, set = null, last, have, onclose, onwrote, preset }: Props = $props();

  const KG = 1000;
  let name = $state(untrack(() => String(set?.props.exercise ?? set?.title ?? preset?.name ?? '')));
  // **Nothing here is pre-filled with a number of our own.** How many sets, how many reps and how
  // heavy is advice, and advice needs a source (`decisions.md#data`, 2026-10-11, *nothing is
  // invented*). The fields start from the set being changed, or from what this exercise was last
  // time once it is chosen — and otherwise empty.
  let sets = $state(1);
  let reps = $state(untrack(() => whole(set?.props.reps) ?? 0));
  let kg = $state(untrack(() => (whole(set?.props.load_g) ?? 0) / KG));
  let busy = $state(false);
  let failed = $state('');

  // ── an activity instead of sets ────────────────────────────────────────────────────────────────
  /// Is the entry being changed an activity? Then this sheet changes its minutes and effort.
  const isActivity = (o: ObjectMeta | null) => !!o && o.props.activity !== undefined;
  let mode = $state<'sets' | 'activity'>(untrack(() => (isActivity(set) ? 'activity' : 'sets')));
  let activity = $state(untrack(() => String(set?.props.activity ?? '')));
  let countsAs = $state(untrack(() => String(set?.props.counts_as ?? 'aerobic')));
  let minutes = $state(untrack(() => whole(set?.props.minutes) ?? 0));
  let effort = $state(untrack(() => whole(set?.props.effort) ?? 0));
  /// Activities a known body names, with what each counts as (`plan_library`). Offered, never
  /// required: a name is whatever the person types.
  let known = $state<PlanActivity[]>([]);
  $effect(() => {
    planLibrary('fitness')
      .then((l) => (known = l.activities))
      .catch(() => (known = []));
  });
  const KINDS: [string, string][] = [
    ['aerobic', 'Aerobic'],
    ['strength', 'Strengthening'],
    ['balance', 'Balance'],
    ['flexibility', 'Stretching'],
  ];
  function pickActivity(a: PlanActivity) {
    activity = a.name;
    countsAs = a.kind;
    // A place to start, where a source gives one; the note under the fields says whose.
    if (a.minutes) minutes = a.minutes;
    if (a.effort) effort = a.effort;
    start = a.basis ?? '';
  }

  // ── choosing from a list, then changing the numbers ────────────────────────────────────────────
  // **What is added is picked, not typed** (`decisions.md#ui`, 2026-10-12, *Fitness is the week,
  // the exercises and the regime*): what was done before, the activities a body names, and the
  // exercise cards. It arrives with a place to start — what it was last time, or its source's own
  // figure, or the general one — and the person changes it. Typing a name is still there.
  let picked = $state(untrack(() => !!set || !!preset));
  let seek = $state('');
  let cards = $state<ExerciseCard[]>([]);
  $effect(() => {
    void cardLibrary()
      .then((l) => (cards = l.exercises))
      .catch(() => (cards = []));
  });
  /// Whose the starting numbers are, said under the fields.
  let start = $state('');
  /// Opened on an exercise from its card: the same place to start as picking it from the list.
  let presetDone = false;
  $effect(() => {
    if (presetDone || !preset || set || !cards.length) return;
    presetDone = true;
    const card = cards.find((c) => sameName(c.name, preset.name));
    if (card) pickCard(card);
  });
  const GYM: ExerciseCard['equipment'][] = ['bar', 'free weights', 'machine'];
  let where = $state<'all' | 'none' | 'gym'>('all');
  const found = $derived.by(() => {
    const text = seek.trim().toLowerCase();
    const has = (n: string) => !text || n.toLowerCase().includes(text);
    return {
      before: last.filter((r) => has(r.title)).slice(0, 6),
      activities: known.filter((a) => has(a.name)),
      cards: cards
        .filter((c) => !['flexibility', 'warm-up'].includes(c.area) || !!text)
        .filter((c) => has(c.name))
        .filter((c) =>
          where === 'all'
            ? true
            : where === 'gym'
              ? GYM.includes(c.equipment)
              : !GYM.includes(c.equipment),
        )
        .slice(0, text ? 40 : 14),
    };
  });
  /// A card's own line, read for its numbers: "3 sets of 5 to 10 repetitions", "Repeat 5 times".
  function doseOf(line: string | undefined): { sets: number; reps: number } | null {
    if (!line) return null;
    const many = /(\d+)\s+sets?\s+of\s+(\d+)/i.exec(line);
    if (many) return { sets: Number(many[1]), reps: Number(many[2]) };
    const once = /(?:repeat(?: the exercise)?|aim for|perform|do)\s+(\d+)/i.exec(line);
    return once ? { sets: 1, reps: Number(once[1]) } : null;
  }
  function pickCard(c: ExerciseCard) {
    mode = 'sets';
    name = c.name;
    region = ['upper-push', 'upper-pull', 'lower-push', 'lower-pull'].includes(c.area)
      ? c.area
      : '';
    const before = last.find((r) => sameName(r.title, c.name));
    const own = doseOf(c.dose);
    if (before && whole(before.props.reps) !== null) {
      choose(before);
      start = 'As it was the last time you did it.';
    } else if (own) {
      sets = own.sets;
      reps = own.reps;
      start = `Its card says: “${c.dose}” Change it to what you do.`;
    } else {
      // The general figure, where the card gives none of its own.
      sets = 2;
      reps = 8;
      start =
        'A place to start, from the NHS: “Try to do 8 to 12 repetitions … Try to do at least 2 sets”. Change it to what you do.';
    }
    picked = true;
  }
  function pickBefore(r: RecentItem) {
    if (r.props.activity !== undefined) {
      mode = 'activity';
      activity = String(r.props.activity);
      countsAs = String(r.props.counts_as ?? 'aerobic');
      minutes = whole(r.props.minutes) ?? 0;
      effort = whole(r.props.effort) ?? 0;
    } else {
      mode = 'sets';
      choose(r);
    }
    start = 'As it was the last time you did it.';
    picked = true;
  }
  /// WHO's own wording for the scale (2020 guidelines): moderate is "a 5 or 6", vigorous "a 7 or
  /// 8", of 10. The talk test is the NHS's.
  const effortSays = $derived(
    effort >= 7
      ? 'Vigorous: you cannot say more than a few words without pausing for breath. A minute counts as two.'
      : effort >= 5
        ? 'Moderate: you can still talk, but not sing.'
        : 'Lighter than moderate: it is recorded, and does not count towards weekly minutes.',
  );
  const activityReady = $derived(activity.trim() !== '' && minutes > 0);

  // ── which region of the body an exercise trains ────────────────────────────────────────────────
  /// ACSM's 2026 position stand names no muscles; four regions are enough to say "all major
  /// muscle groups". Optional: an exercise with none is still a set done.
  const REGIONS: [string, string][] = [
    ['upper-push', 'Upper body, push'],
    ['upper-pull', 'Upper body, pull'],
    ['lower-push', 'Lower body, push'],
    ['lower-pull', 'Lower body, pull'],
  ];
  let region = $state(untrack(() => String(set?.props.region ?? preset?.region ?? '')));

  // What is being planned is kept on the device until it is written (`drafts.ts`): an exercise
  // half chosen survives leaving the sheet or closing the app. Changing a set already there, or
  // starting from an exercise's card, starts from that instead.
  const DRAFT = untrack(() => `workout:${def.log}`);
  type Kept = {
    name: string;
    sets: number;
    reps: number;
    kg: number;
    mode: 'sets' | 'activity';
    activity: string;
    countsAs: string;
    minutes: number;
    effort: number;
    region: string;
    picked: boolean;
  };
  const fresh = untrack(() => !set && !preset);
  let resumed = $state(false);
  if (fresh) {
    const was = draftOf<Kept>(DRAFT);
    if (was && (was.name || was.activity || was.reps || was.minutes)) {
      name = was.name ?? '';
      sets = was.sets || 1;
      reps = was.reps || 0;
      kg = was.kg || 0;
      mode = was.mode === 'activity' ? 'activity' : 'sets';
      activity = was.activity ?? '';
      countsAs = was.countsAs || 'aerobic';
      minutes = was.minutes || 0;
      effort = was.effort || 0;
      region = was.region ?? '';
      picked = !!was.picked;
      resumed = true;
    }
  }
  $effect(() => {
    const now: Kept = {
      name,
      sets,
      reps,
      kg,
      mode,
      activity,
      countsAs,
      minutes,
      effort,
      region,
      picked,
    };
    if (fresh) keepDraft(DRAFT, now);
  });
  function afresh() {
    name = activity = region = '';
    sets = 1;
    reps = kg = minutes = effort = 0;
    countsAs = 'aerobic';
    picked = false;
    resumed = false;
  }

  /// What was done before that is an exercise: an activity is offered under activities.
  const exercises = $derived(last.filter((r) => r.props.activity === undefined));
  /// Choosing an exercise done before brings back what it was last time.
  function choose(r: RecentItem) {
    name = r.title;
    reps = whole(r.props.reps) ?? reps;
    kg = (whole(r.props.load_g) ?? kg * KG) / KG;
    region = String(r.props.region ?? region);
  }
  const ready = $derived(name.trim() !== '' && reps > 0 && kg >= 0 && sets > 0);
  const step = (v: number, by: number, min: number) =>
    Math.max(min, Math.round((v + by) * 10) / 10);

  async function write(fn: () => Promise<void>) {
    if (busy) return;
    busy = true;
    failed = '';
    try {
      await fn();
    } catch (e) {
      failed = String(e);
    } finally {
      busy = false;
    }
  }

  const add = () =>
    write(async () => {
      const exercise = name.trim();
      const after = Math.max(
        0,
        ...have.filter((s) => s.props.exercise === exercise).map((s) => whole(s.props.set) ?? 0),
      );
      const drafts: NoteDraft[] = Array.from({ length: sets }, (_, i) => ({
        title: exercise,
        start: day,
        props: {
          log: def.log,
          exercise,
          set: after + i + 1,
          reps,
          load_g: Math.round(kg * KG),
          planned: 1,
          ...(region ? { region } : {}),
        },
      }));
      await createNotesUndoable(drafts, `planned ${exercise}`);
      dropDraft(DRAFT);
      onwrote();
      onclose();
    });

  /// An activity is written as done: there is nothing to tick afterwards.
  const addActivity = () =>
    write(async () => {
      const what = activity.trim();
      await createNotesUndoable(
        [
          {
            title: what,
            start: day,
            props: { log: def.log, activity: what, counts_as: countsAs, minutes, effort },
          },
        ],
        `logged ${what}`,
      );
      dropDraft(DRAFT);
      onwrote();
      onclose();
    });
  const saveActivity = () =>
    write(async () => {
      if (!set) return;
      await setPropertiesUndoable(
        set.id,
        { minutes, effort, counts_as: countsAs },
        `changed ${set.title ?? 'an activity'}`,
      );
      onwrote();
      onclose();
    });

  const save = () =>
    write(async () => {
      if (!set) return;
      const load_g = Math.round(kg * KG);
      await setPropertiesUndoable(
        set.id,
        // A set already ticked keeps its work in step with its numbers.
        { reps, load_g, region, ...(set.props.planned ? {} : { work_g: reps * load_g }) },
        `changed a set of ${set.title ?? 'an exercise'}`,
      );
      onwrote();
      onclose();
    });

  const remove = () =>
    write(async () => {
      if (!set) return;
      const title = isActivity(set)
        ? (set.title ?? 'an activity')
        : `set ${whole(set.props.set) ?? ''} of ${set.title ?? 'an exercise'}`;
      await deleteUndoable(set.id, title);
      onwrote({ removed: title });
      onclose();
    });
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<div class="workout-overlay">
  <button type="button" class="scrim" onclick={onclose} aria-label="close" tabindex="-1"></button>
  <div
    class="sheet"
    role="dialog"
    aria-modal="true"
    aria-label={set
      ? isActivity(set)
        ? 'change an activity'
        : 'change a set'
      : 'add to the session'}
  >
    <header>
      <h2>
        {set
          ? isActivity(set)
            ? activity
            : `${name} · set ${whole(set.props.set) ?? ''}`
          : 'Add to the session'}
      </h2>
      <button type="button" class="x" onclick={onclose} aria-label="close">
        <Icon name="close" size={18} />
      </button>
    </header>

    {#if !set && !picked}
      <!-- Pick, then change the numbers. -->
      <input
        class="seek"
        type="search"
        placeholder="An exercise or an activity"
        aria-label="search what to add"
        bind:value={seek}
        spellcheck="false"
      />
      <div class="chips" role="group" aria-label="where">
        {#each [['all', 'All'], ['none', 'No equipment'], ['gym', 'Gym']] as [k, label] (k)}
          <button
            type="button"
            class="chip"
            class:on={where === k}
            aria-pressed={where === k}
            onclick={() => (where = k as typeof where)}>{label}</button
          >
        {/each}
      </div>
      {#if found.before.length}
        <h3>Done before</h3>
        <ul class="pick">
          {#each found.before as r (r.title)}
            <li>
              <button type="button" onclick={() => pickBefore(r)}>
                <span>{r.title}</span>
                <span class="sub"
                  >{r.props.activity !== undefined
                    ? `${whole(r.props.minutes) ?? 0} min`
                    : `${whole(r.props.reps) ?? 0} reps`}</span
                >
              </button>
            </li>
          {/each}
        </ul>
      {/if}
      {#if found.activities.length}
        <h3>Activities, by the minute</h3>
        <ul class="pick">
          {#each found.activities as a (a.name)}
            <li>
              <button
                type="button"
                onclick={() => ((mode = 'activity'), pickActivity(a), (picked = true))}
              >
                <span>{a.name}</span>
                <span class="sub">{a.minutes ? `${a.minutes} min to start` : a.kind}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
      {#if found.cards.length}
        <h3>Exercises, in sets</h3>
        <ul class="pick">
          {#each found.cards as c (c.id)}
            <li>
              <button type="button" onclick={() => pickCard(c)}>
                <span>{c.name}</span>
                <span class="sub">{c.equipment === 'none' ? 'no equipment' : c.equipment}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
      <button type="button" class="quiet" onclick={() => ((picked = true), (start = ''))}>
        Not in the list: type a name
      </button>
    {:else}
      {#if !set && !preset}
        <button type="button" class="quiet" onclick={() => (picked = false)}>Choose another</button>
      {/if}
      {#if start}<p class="says" role="note">{start}</p>{/if}
      {#if !set}
        <div class="modes" role="group" aria-label="what to add">
          <button type="button" class:on={mode === 'sets'} onclick={() => (mode = 'sets')}
            >Exercise, in sets</button
          >
          <button type="button" class:on={mode === 'activity'} onclick={() => (mode = 'activity')}
            >Activity, in minutes</button
          >
        </div>
      {/if}

      {#if !set && mode === 'sets'}
        <label class="field">
          <span>Exercise</span>
          <input type="text" bind:value={name} autocomplete="off" />
        </label>
        {#if exercises.length}
          <div class="chips">
            {#each exercises.slice(0, 8) as r (r.title)}
              <button
                type="button"
                class="chip"
                class:on={r.title === name}
                onclick={() => choose(r)}>{r.title}</button
              >
            {/each}
          </div>
        {/if}
      {/if}

      {#snippet stepper(
        label: string,
        value: number,
        by: number,
        min: number,
        put: (v: number) => void,
      )}
        <div class="row">
          <span class="lbl">{label}</span>
          <button
            type="button"
            class="round"
            aria-label={`less ${label}`}
            onclick={() => put(step(value, -by, min))}
          >
            <Icon name="minus" size={20} />
          </button>
          <input
            type="number"
            inputmode="decimal"
            aria-label={label}
            {min}
            {value}
            oninput={(e) => put(Number((e.currentTarget as HTMLInputElement).value) || min)}
          />
          <button
            type="button"
            class="round"
            aria-label={`more ${label}`}
            onclick={() => put(step(value, by, min))}
          >
            <Icon name="plus" size={20} />
          </button>
        </div>
      {/snippet}

      {#if mode === 'activity'}
        {#if !set}
          <label class="field">
            <span>Activity</span>
            <input type="text" bind:value={activity} autocomplete="off" />
          </label>
          {#if known.length}
            <div class="chips">
              {#each known as a (a.name)}
                <button
                  type="button"
                  class="chip"
                  class:on={a.name === activity}
                  title={`${a.by} counts it as ${a.kind}`}
                  onclick={() => pickActivity(a)}>{a.name}</button
                >
              {/each}
            </div>
          {/if}
        {/if}
        <div class="field">
          <span>It counts as</span>
          <div class="chips" role="radiogroup" aria-label="what it counts as">
            {#each KINDS as [v, label] (v)}
              <button
                type="button"
                role="radio"
                class="chip"
                class:on={countsAs === v}
                aria-checked={countsAs === v}
                onclick={() => (countsAs = v)}>{label}</button
              >
            {/each}
          </div>
        </div>
        {@render stepper('Minutes', minutes, 5, 0, (v) => (minutes = Math.round(v)))}
        {@render stepper(
          'Effort (0–10)',
          effort,
          1,
          0,
          (v) => (effort = Math.min(10, Math.round(v))),
        )}
        <p class="says">{effortSays}</p>
      {:else}
        {#if !set}{@render stepper(
            'Sets',
            sets,
            1,
            1,
            (v) => (sets = Math.min(Math.round(v), 12)),
          )}{/if}
        {@render stepper('Reps', reps, 1, 0, (v) => (reps = Math.round(v)))}
        {@render stepper('Load (kg)', kg, 2.5, 0, (v) => (kg = v))}
        <div class="field">
          <span>It trains</span>
          <div class="chips" role="radiogroup" aria-label="the region it trains">
            {#each REGIONS as [v, label] (v)}
              <button
                type="button"
                role="radio"
                class="chip"
                class:on={region === v}
                aria-checked={region === v}
                onclick={() => (region = region === v ? '' : v)}>{label}</button
              >
            {/each}
          </div>
        </div>
      {/if}

      {#if resumed}<Resumed onafresh={afresh} />{/if}
      {#if failed}<p class="failed" role="alert">{failed}</p>{/if}

      {#if set}
        <div class="pair">
          <button type="button" class="quiet" onclick={remove} disabled={busy}>Remove</button>
          {#if mode === 'activity'}
            <button type="button" class="go" onclick={saveActivity} disabled={busy || minutes <= 0}
              >Save</button
            >
          {:else}
            <button type="button" class="go" onclick={save} disabled={busy || !ready}>Save</button>
          {/if}
        </div>
      {:else if mode === 'activity'}
        <button type="button" class="go" onclick={addActivity} disabled={busy || !activityReady}>
          Log {minutes} min
        </button>
      {:else}
        <button type="button" class="go" onclick={add} disabled={busy || !ready}>
          Add {sets} × {reps}
        </button>
      {/if}
    {/if}
  </div>
</div>

<style>
  .workout-overlay {
    position: fixed;
    inset: 0;
    z-index: 80;
    display: flex;
    justify-content: center;
    align-items: flex-end;
    box-sizing: border-box;
    height: var(--app-h);
    padding-top: calc(var(--overlay-inset) + var(--safe-top));
  }
  .scrim {
    position: fixed;
    inset: 0;
    border: 0;
    background: rgb(0 0 0 / 0.45);
    cursor: default;
  }
  .sheet {
    position: relative;
    width: min(34rem, 100%);
    max-height: 100%;
    overflow-y: auto;
    overscroll-behavior: contain;
    box-sizing: border-box;
    padding: var(--space-4) var(--space-4) max(var(--safe-bottom), var(--space-4));
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    background: var(--surface-elevated);
    border-radius: var(--radius-lg) var(--radius-lg) 0 0;
    box-shadow: var(--shadow-lg);
    animation: sheet-up var(--dur-med) var(--ease);
  }
  @keyframes sheet-up {
    from {
      transform: translateY(100%);
    }
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  h2 {
    margin: 0;
    font-size: var(--text-lg);
  }
  button,
  input {
    font: inherit;
    color: inherit;
  }
  .x {
    width: 2.75rem;
    height: 2.75rem;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: var(--radius-pill);
    background: none;
    color: var(--text-muted);
    cursor: pointer;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .field input {
    min-height: 2.75rem;
    padding: 0 var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
  }
  /* Two kinds of thing a session holds; one is on screen at a time. */
  .modes {
    display: flex;
    padding: 2px;
    border-radius: var(--radius-pill);
    background: var(--surface-hover);
  }
  .modes button {
    flex: 1;
    min-height: 2.5rem;
    border: 0;
    border-radius: var(--radius-pill);
    background: none;
    color: var(--text-muted);
    cursor: pointer;
  }
  .modes button.on {
    background: var(--surface-elevated);
    color: var(--text);
    box-shadow: var(--shadow-sm);
  }
  .says {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
    line-height: 1.45;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  .chip {
    min-height: 2.25rem;
    padding: 0 var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    background: none;
    cursor: pointer;
  }
  .chip.on {
    border-color: var(--accent);
    background: var(--accent-subtle);
  }
  .row {
    display: grid;
    grid-template-columns: 1fr auto 5rem auto;
    align-items: center;
    gap: var(--space-2);
  }
  .lbl {
    color: var(--text-muted);
  }
  .row input {
    width: 100%;
    box-sizing: border-box;
    min-height: 2.75rem;
    border: 0;
    background: none;
    text-align: center;
    font-size: var(--text-xl);
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    -moz-appearance: textfield;
    appearance: textfield;
  }
  .row input::-webkit-outer-spin-button,
  .row input::-webkit-inner-spin-button {
    -webkit-appearance: none;
  }
  .round {
    width: 2.75rem;
    height: 2.75rem;
    display: grid;
    place-items: center;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-pill);
    background: var(--surface);
    cursor: pointer;
    touch-action: manipulation;
  }
  .failed {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--danger-fg);
  }
  .pair {
    display: flex;
    gap: var(--space-2);
  }
  .go,
  .quiet {
    min-height: 3rem;
    border-radius: var(--radius-md);
    cursor: pointer;
  }
  .go {
    flex: 1;
    border: 0;
    background: var(--accent);
    color: var(--accent-contrast);
    font-weight: 600;
  }
  .go:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .quiet {
    padding: 0 var(--space-4);
    border: 1px solid var(--border);
    background: none;
  }
  .seek {
    min-height: 2.75rem;
    padding: 0 var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: inherit;
    font: inherit;
  }
  h3 {
    margin: var(--space-2) 0 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
    font-weight: 600;
  }
  .pick {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  .pick button {
    width: 100%;
    box-sizing: border-box;
    min-height: 2.75rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    padding: 0 var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: none;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .sub {
    flex: none;
    font-size: var(--text-xs);
    color: var(--text-muted);
  }
</style>
