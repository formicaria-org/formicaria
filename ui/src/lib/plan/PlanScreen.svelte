<script lang="ts">
  // The screen where a diet or a training regime is chosen, fine-tuned and followed
  // (`decisions.md#ui`, 2026-10-12). It shows one of three things and nothing else:
  //
  // - **what is being followed**, measured for this week, when something is;
  // - **the library**, when nothing is, or when the person asks to change;
  // - **one option**, opened from the library: what it would hold them to, whose number each is,
  //   every number changeable, and a button to follow it.
  //
  // The library, the formulas and the measuring are the core's (`plan.rs`); this draws what it
  // answers. Following writes one note; stopping gives that note a last day.
  import ExampleDay from './ExampleDay.svelte';
  import Resumed from '../Resumed.svelte';
  import { draftOf, dropDraft, keepDraft } from '../drafts';
  import ExampleWeek from './ExampleWeek.svelte';
  import { onMount, untrack } from 'svelte';
  import Icon from '../Icon.svelte';
  import { planLibrary, planPreview, planProgress } from '../ipc';
  import { createNotesUndoable, deleteUndoable, setPropertiesUndoable } from '../undo.svelte';
  import { addDays, dayLabel, dayLetter } from '../units';
  import {
    boundProps,
    fraction,
    planBody,
    shownAsk,
    shownBound,
    shownValue,
    storedBound,
    unitOf,
    verdict,
  } from './measure';
  import type {
    PlanCheck,
    PlanLibrary,
    PlanOption,
    PlanPreview,
    PlanProfile,
    PlanProgress,
    TotalsDef,
  } from '../types';

  interface Props {
    def: TotalsDef;
    today: string;
    /** Anything the app says changed: ask again. */
    signal?: unknown;
    onsaved: () => void;
  }
  let { def, today, signal, onsaved }: Props = $props();

  /// Which library: the food log's is diets, the training log's is regimes.
  const group = $derived(def.add === 'workout' ? 'fitness' : 'food');
  const noun = $derived(group === 'fitness' ? 'regime' : 'diet');
  const program = $derived(group === 'fitness' ? 'training' : 'food');

  let progress = $state<PlanProgress | null>(null);
  let library = $state<PlanLibrary | null>(null);
  /// The library is on screen although something is followed: the person asked to change.
  let choosing = $state(false);
  let open = $state<PlanPreview | null>(null);
  let failed = $state('');
  let busy = $state(false);

  let asked = 0;
  async function load() {
    const mine = ++asked;
    try {
      const [p, l] = await Promise.all([
        planProgress(group, today),
        library ? Promise.resolve(library) : planLibrary(group),
      ]);
      if (mine !== asked) return;
      progress = p;
      library = l;
      failed = '';
    } catch (e) {
      if (mine === asked) failed = String(e);
    }
  }
  $effect(() => {
    void signal;
    void group;
    untrack(() => void load());
  });

  // ── the library ────────────────────────────────────────────────────────────────────────────────
  /// What an option rests on, said before its name: a guideline is not a single trial, and the
  /// list says which is which (`sources-2026-10-11.md` ranks them the same way).
  const RESTS: [PlanOption['rests_on'], string][] = [
    ['guideline', 'From a guideline'],
    ['position', 'From a position stand or an expert consensus'],
    ['review', 'From a review of trials'],
    ['definition', 'Defined by a consensus, recommended by no public body'],
    ['described', 'Set out by its own body; not measurable from what you log'],
    ['own', 'Yours'],
  ];
  const shelves = $derived(
    RESTS.map(([rests, heading]) => ({
      heading,
      plans: (library?.plans ?? []).filter((p) => p.rests_on === rests),
    })).filter((s) => s.plans.length),
  );

  // ── one option, opened ─────────────────────────────────────────────────────────────────────────
  /// The bounds as typed, by check: text, so an empty field stays empty.
  let fields = $state<Record<string, { min: string; max: string }>>({});
  /// The measurements as typed.
  let me = $state<{ sex: string; born: string; height: string; weight: string; pal: string }>({
    sex: '',
    born: '',
    height: '',
    weight: '',
    pal: '',
  });

  function fill(p: PlanPreview, from?: Record<string, { min?: number; max?: number }>) {
    const bounds = from ?? p.bounds;
    fields = Object.fromEntries(
      p.plan.checks.map((c) => [
        c.id,
        { min: shownBound(c, bounds[c.id]?.min), max: shownBound(c, bounds[c.id]?.max) },
      ]),
    );
    const pr = p.profile;
    me = {
      sex: pr.sex ?? '',
      born: pr.born ? String(pr.born) : '',
      height: pr.height_cm ? String(pr.height_cm) : '',
      weight: pr.weight_g ? String(Math.round(pr.weight_g / 100) / 10) : '',
      pal: pr.pal_x10 ? String(pr.pal_x10) : '',
    };
  }
  async function look(id: string, keep?: Record<string, { min?: number; max?: number }>) {
    failed = '';
    try {
      const p = await planPreview(id, today);
      fill(p, keep);
      open = p;
    } catch (e) {
      failed = String(e);
    }
  }
  // The option that is open, with its numbers and the measurements as typed, is kept on the
  // device until it is followed or left by its own Back (`drafts.ts`): fine-tuning a diet and
  // then changing view, or closing the app, comes back to it as it was.
  // Named once: the screen is of one group for as long as it is on screen.
  const DRAFT = untrack(() => `plan:${program}`);
  type Kept = { id: string; fields: typeof fields; me: typeof me };
  let resumed = $state(false);
  $effect(() => {
    if (open) keepDraft(DRAFT, { id: open.plan.id, fields, me } satisfies Kept);
  });
  onMount(async () => {
    const was = draftOf<Kept>(DRAFT);
    if (!was?.id) return;
    await look(was.id);
    if (open?.plan.id !== was.id) return void dropDraft(DRAFT);
    // Only the lines this option still has; what it has gained since keeps its own number.
    for (const k of Object.keys(fields)) if (was.fields?.[k]) fields[k] = was.fields[k];
    me = { ...me, ...was.me };
    resumed = true;
  });
  function leave() {
    dropDraft(DRAFT);
    open = null;
    resumed = false;
  }
  async function afresh() {
    const id = open?.plan.id;
    resumed = false;
    if (id) await look(id);
  }
  /// Change the numbers of what is followed: the same form, starting from the note's own bounds.
  function retune() {
    const p = progress;
    if (!p?.plan) return;
    const mine = Object.fromEntries(p.checks.map((c) => [c.id, { min: c.min, max: c.max }]));
    void look(p.plan.id, mine);
  }

  const settled = $derived.by(() => {
    const out: Record<string, { min?: number; max?: number }> = {};
    for (const c of open?.plan.checks ?? []) {
      const f = fields[c.id];
      const b = { min: storedBound(c, f?.min), max: storedBound(c, f?.max) };
      if (b.min !== undefined || b.max !== undefined) out[c.id] = b;
    }
    return out;
  });
  /// A plan with nothing to measure and nothing to keep to is not something to follow.
  const ready = $derived(Object.keys(settled).length > 0 && !(open?.missing.length ?? 0));

  const LEVELS: [string, string][] = [
    ['14', 'Low active (sedentary)'],
    ['16', 'Moderately active'],
    ['18', 'Active'],
    ['20', 'Very active'],
  ];
  const needs = (f: string) => !!open?.plan.needs.includes(f);

  async function saveMe() {
    if (busy || !open) return;
    const props: Record<string, string | number> = {};
    if (me.sex) props.sex = me.sex;
    const born = Number(me.born);
    if (Number.isInteger(born) && born > 1900) props.born = born;
    const height = Number(me.height);
    if (Number.isFinite(height) && height > 0) props.height_cm = Math.round(height);
    const weight = Number(me.weight);
    if (Number.isFinite(weight) && weight > 0) props.weight_g = Math.round(weight * 1000);
    if (me.pal) props.pal_x10 = Number(me.pal);
    busy = true;
    failed = '';
    try {
      const id = open.profile.id;
      if (id) await setPropertiesUndoable(id, props, 'changed your measurements');
      else
        await createNotesUndoable(
          [
            {
              title: 'About me',
              body: 'Your own measurements, used only to work out numbers you ask for.\n',
              props: { profile: 'me', ...props },
            },
          ],
          'wrote down your measurements',
        );
      // A weight is also a weighing: an entry of today, so it is one of the trends, and so the
      // latest weighing — which is what the numbers are worked out from — is this one.
      if (typeof props.weight_g === 'number' && props.weight_g !== open.profile.weight_g)
        await createNotesUndoable(
          [{ title: 'Weight', start: today, props: { log: 'body', weight_g: props.weight_g } }],
          'wrote down your weight',
        );
      onsaved();
      // The numbers follow the measurements: work them out again.
      await look(open.plan.id);
    } catch (e) {
      failed = String(e);
    } finally {
      busy = false;
    }
  }

  async function follow() {
    const p = open;
    if (busy || !p || !ready) return;
    busy = true;
    failed = '';
    try {
      const lines = p.plan.checks
        .filter((c) => settled[c.id])
        .map((c) => ({
          label: c.label,
          ask: shownAsk({ unit: unitFor(c), ...settled[c.id] }),
          basis: c.basis,
        }));
      await createNotesUndoable(
        [
          {
            title: p.plan.name,
            start: today,
            body: planBody(p.plan, lines),
            props: {
              program,
              plan: p.plan.id,
              ...boundProps(
                p.plan.checks,
                settled,
                def.metrics.map((m) => m.key),
              ),
              ...(p.energy ? { p_energy: p.energy } : {}),
            },
          },
        ],
        `followed ${p.plan.name}`,
      );
      dropDraft(DRAFT);
      resumed = false;
      open = null;
      choosing = false;
      onsaved();
      await load();
    } catch (e) {
      failed = String(e);
    } finally {
      busy = false;
    }
  }
  /// The unit a check's measured value is in, as the core names it.
  const unitFor = (c: PlanCheck) =>
    c.kind === 'sum'
      ? (c.key ?? '')
      : c.kind === 'share'
        ? 'percent'
        : c.kind === 'minutes'
          ? 'minutes'
          : c.kind === 'sets'
            ? 'sets'
            : 'days';

  // ── what is followed ───────────────────────────────────────────────────────────────────────────
  const week = $derived(
    progress ? Array.from({ length: 7 }, (_, i) => addDays(progress!.from, i)) : [],
  );
  const weekOver = $derived(!!progress && today >= progress.to);

  async function stop() {
    const note = progress?.note;
    if (busy || !note) return;
    busy = true;
    try {
      // Its last day is yesterday: today is measured against whatever was followed before, or
      // nothing.
      await setPropertiesUndoable(
        note.id,
        { due: addDays(today, -1) },
        `stopped following ${note.title ?? 'a plan'}`,
      );
      onsaved();
      await load();
    } catch (e) {
      failed = String(e);
    } finally {
      busy = false;
    }
  }
  async function toggleKept() {
    const p = progress;
    if (busy || !p?.plan) return;
    busy = true;
    try {
      if (p.kept_today) await deleteUndoable(p.kept_today, `kept to ${p.plan.name}`);
      else
        await createNotesUndoable(
          [
            {
              title: `Kept to ${p.plan.name}`,
              start: today,
              props: { log: 'kept', plan: p.plan.id },
            },
          ],
          `said you kept to ${p.plan.name}`,
        );
      onsaved();
      await load();
    } catch (e) {
      failed = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="plan">
  {#if failed}<p class="failed" role="alert">{failed}</p>{/if}

  {#if open}
    {@const p = open.plan}
    <button type="button" class="back" onclick={leave}>
      <Icon name="chevronLeft" size={18} /> All {noun}s
    </button>
    {#if resumed}<Resumed onafresh={afresh} />{/if}
    <header class="head">
      <h2>{p.name}</h2>
      <p>{p.summary}</p>
    </header>
    {#if p.caution}<p class="caution">{p.caution}</p>{/if}

    {#if p.needs.length}
      <section class="card">
        <h3>About you</h3>
        <p class="why">
          Used only to work out the numbers below. Kept as a note in your vault, called “About me”.
        </p>
        {#if needs('sex')}
          <div class="field">
            <span>Sex</span>
            <div class="seg" role="group" aria-label="sex">
              {#each [['female', 'Female'], ['male', 'Male']] as [v, label] (v)}
                <button
                  type="button"
                  class:on={me.sex === v}
                  aria-pressed={me.sex === v}
                  onclick={() => (me.sex = v)}>{label}</button
                >
              {/each}
            </div>
          </div>
        {/if}
        {#if needs('born')}
          <label class="field">
            <span>Year of birth</span>
            <input type="number" inputmode="numeric" bind:value={me.born} placeholder="1990" />
          </label>
        {/if}
        {#if needs('height_cm')}
          <label class="field">
            <span>Height, cm</span>
            <input type="number" inputmode="decimal" bind:value={me.height} />
          </label>
        {/if}
        {#if needs('weight_g')}
          <label class="field">
            <span>Weight, kg</span>
            <input type="number" inputmode="decimal" bind:value={me.weight} />
          </label>
        {/if}
        {#if needs('pal_x10')}
          <div class="field tall">
            <span>How active you are</span>
            <div class="levels" role="radiogroup" aria-label="how active you are">
              {#each LEVELS as [v, label] (v)}
                <button
                  type="button"
                  role="radio"
                  class:on={me.pal === v}
                  aria-checked={me.pal === v}
                  onclick={() => (me.pal = v)}>{label}</button
                >
              {/each}
            </div>
          </div>
          <p class="why">EFSA’s four activity levels, which it calls “a rough indication”.</p>
        {/if}
        <button type="button" class="quiet" onclick={saveMe} disabled={busy}>
          {open.missing.length ? 'Work out my numbers' : 'Update my numbers'}
        </button>
        {#if open.energy}
          <p class="estimate">
            Estimated energy need: <strong>{shownValue({ unit: 'kcal' }, open.energy)}</strong> a day.
          </p>
        {/if}
      </section>
    {/if}

    {#if p.describes?.length}
      <section class="card">
        <h3>What it asks, in its own terms</h3>
        <ul class="plain">
          {#each p.describes as d (d)}<li>{d}</li>{/each}
        </ul>
      </section>
    {/if}

    <section class="card">
      <h3>What you will be held to</h3>
      <p class="why">
        {p.rests_on === 'own'
          ? 'Fill in what you want measured; leave the rest empty.'
          : 'Each number is the source’s. Change any of them, or empty one to leave it out.'}
      </p>
      {#each p.checks as c (c.id)}
        {@const u = unitOf(c).unit}
        <div class="check">
          <div class="what">
            <strong>{c.label}</strong>
            {#if p.rests_on !== 'own'}<span class="basis">{c.basis}</span>{/if}
          </div>
          <div class="bounds">
            <label>
              <span>at least</span>
              <input
                type="number"
                inputmode="decimal"
                aria-label={`${c.label}, at least`}
                bind:value={fields[c.id].min}
              />
            </label>
            <label>
              <span>at most</span>
              <input
                type="number"
                inputmode="decimal"
                aria-label={`${c.label}, at most`}
                bind:value={fields[c.id].max}
              />
            </label>
            <span class="unit">{u}</span>
          </div>
        </div>
      {/each}
    </section>

    {#if p.unmeasured.length}
      <section class="card">
        <h3>What formicaria cannot measure</h3>
        <ul class="plain">
          {#each p.unmeasured as u (u)}<li>{u}</li>{/each}
        </ul>
      </section>
    {/if}

    {@render sources(p)}

    {#each p.examples ?? [] as example, i (i)}
      {#if 'week' in example}
        <ExampleWeek {example} {onsaved} />
      {:else}
        <ExampleDay {example} {today} {onsaved} />
      {/if}
    {/each}
    {#if open.missing.length}
      <p class="why">Fill in “About you” to work out the numbers this needs.</p>
    {/if}
    <button type="button" class="go" onclick={follow} disabled={busy || !ready}>
      Follow this
    </button>
  {:else if progress?.plan && !choosing}
    {@const p = progress.plan}
    <header class="head">
      <h2>{p.name}</h2>
      <p>
        Following since {dayLabel(progress.note?.start?.slice(0, 10) ?? today)} · week of {dayLabel(
          progress.from,
        )}
      </p>
    </header>

    {#if progress.checks.length === 0}
      <p class="none">Nothing in it is being measured. Change its numbers to add something.</p>
    {/if}
    {#each progress.checks as c (c.id)}
      <section class="row" data-state={c.state}>
        <div class="line">
          <strong>{c.label}</strong>
          <span class="ask">{shownAsk(c)}</span>
        </div>
        <div class="line">
          <span class="value">
            {c.value === undefined ? '—' : shownValue(c, c.value)}
            {#if c.per === 'day'}<small>today</small>{/if}
          </span>
          <span class="verdict">{verdict(c, weekOver)}</span>
        </div>
        <div class="bar" role="img" aria-label={`${c.label}: ${verdict(c, weekOver)}`}>
          <div class="fill" style="width:{fraction(c) * 100}%"></div>
        </div>
        <ol class="days" aria-label="this week, day by day">
          {#each week as d, i (d)}
            <li class:has={c.days[i] !== null && c.days[i] !== 0} class:today={d === today}>
              <span>{dayLetter(d)}</span>
            </li>
          {/each}
        </ol>
        {#if c.kind === 'kept'}
          <button
            type="button"
            class="quiet"
            aria-pressed={!!progress.kept_today}
            onclick={toggleKept}
            disabled={busy}
          >
            {progress.kept_today ? 'Kept to it today ✓' : 'I kept to it today'}
          </button>
        {/if}
      </section>
    {/each}

    {#if p.describes?.length}
      <section class="card">
        <h3>What it asks, in its own terms</h3>
        <ul class="plain">
          {#each p.describes as d (d)}<li>{d}</li>{/each}
        </ul>
      </section>
    {/if}
    {#each p.examples ?? [] as example, i (i)}
      {#if 'week' in example}
        <ExampleWeek {example} {onsaved} />
      {:else}
        <ExampleDay {example} {today} {onsaved} />
      {/if}
    {/each}
    {#if p.guides.length}
      <section class="card">
        <h3>How to do it</h3>
        <ul class="plain">
          {#each p.guides as g (g.url)}
            <li>
              <a href={g.url} target="_blank" rel="noopener noreferrer">{g.label}</a>
              <span class="basis">{g.by}, seen {g.seen}</span>
            </li>
          {/each}
        </ul>
      </section>
    {/if}
    {#if p.unmeasured.length}
      <section class="card">
        <h3>What formicaria cannot measure</h3>
        <ul class="plain">
          {#each p.unmeasured as u (u)}<li>{u}</li>{/each}
        </ul>
      </section>
    {/if}
    {#if p.caution}<p class="caution">{p.caution}</p>{/if}
    {@render sources(p)}

    <div class="actions">
      <button type="button" class="quiet" onclick={retune}>Change its numbers</button>
      <button type="button" class="quiet" onclick={() => (choosing = true)}>
        Choose another
      </button>
      <button type="button" class="quiet" onclick={stop} disabled={busy}>Stop following</button>
    </div>
  {:else if progress?.unknown && !choosing}
    <header class="head">
      <h2>{progress.note?.title ?? 'A plan'}</h2>
      <p>
        This copy of formicaria does not know the plan this note follows (“{progress.unknown}”), so
        it cannot measure it. A newer version may.
      </p>
    </header>
    <button type="button" class="quiet" onclick={() => (choosing = true)}>Choose another</button>
  {:else if library}
    <header class="head">
      <h2>Choose a {noun} to follow</h2>
      <p>Each one says where it comes from. Pick one to see what it would hold you to.</p>
    </header>
    {#if choosing}
      <button type="button" class="back" onclick={() => (choosing = false)}>
        <Icon name="chevronLeft" size={18} /> Back to what you follow
      </button>
    {/if}
    {#each shelves as shelf (shelf.heading)}
      <h3 class="shelf">{shelf.heading}</h3>
      {#each shelf.plans as p (p.id)}
        <button type="button" class="option" onclick={() => look(p.id)}>
          <strong>{p.name}</strong>
          <span>{p.summary}</span>
          {#if p.sources.length}
            <small>{[...new Set(p.sources.map((s) => s.by))].join(' · ')}</small>
          {/if}
        </button>
      {/each}
    {/each}
  {:else if !failed}
    <p class="none">Loading…</p>
  {/if}
</div>

{#snippet sources(p: PlanOption)}
  {#if p.sources.length}
    <section class="card">
      <h3>Sources</h3>
      <ul class="plain">
        {#each p.sources as s (s.title)}
          <li>
            <span class="basis">{s.by}</span>
            {#if s.url}
              <a href={s.url} target="_blank" rel="noopener noreferrer">{s.title}</a>
            {:else}
              {s.title}
            {/if}
          </li>
        {/each}
      </ul>
    </section>
  {/if}
{/snippet}

<style>
  .plan {
    height: 100%;
    overflow-y: auto;
    overscroll-behavior: contain;
    box-sizing: border-box;
    /* The whole pane scrolls, with its bar at the pane's edge; the column is kept to a width
       that reads, by padding and not by a narrower box with a bar of its own in mid-screen. */
    padding: var(--space-3) max(var(--space-3), calc((100% - 36rem) / 2)) var(--space-8);
    scrollbar-width: thin;
    scrollbar-color: var(--border-strong) transparent;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .plan > :global(*) {
    flex-shrink: 0;
  }
  button,
  input {
    font: inherit;
    color: inherit;
  }
  .head h2 {
    margin: 0;
    font-size: var(--text-xl);
  }
  .head p,
  .none,
  .why {
    margin: var(--space-1) 0 0;
    color: var(--text-muted);
    font-size: var(--text-sm);
    line-height: 1.5;
  }
  .failed {
    margin: 0;
    color: var(--danger-fg);
    font-size: var(--text-sm);
  }
  .caution {
    margin: 0;
    padding: var(--space-3);
    border-left: 3px solid var(--tint-c);
    background: var(--surface);
    border-radius: var(--radius-sm);
    font-size: var(--text-sm);
    line-height: 1.5;
  }
  .shelf {
    margin: var(--space-3) 0 0;
    font-size: var(--text-xs);
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-subtle);
  }
  .option {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: var(--space-3) var(--space-4);
    text-align: left;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    cursor: pointer;
  }
  .option:hover {
    border-color: var(--accent);
  }
  .option strong {
    font-size: var(--text-md);
  }
  .option span {
    color: var(--text-muted);
    font-size: var(--text-sm);
    line-height: 1.45;
  }
  .option small {
    color: var(--text-subtle);
    font-size: var(--text-xs);
  }
  .back {
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    min-height: 2.25rem;
    padding: 0 var(--space-2) 0 0;
    border: 0;
    background: none;
    color: var(--text-muted);
    cursor: pointer;
  }
  .card,
  .row {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
  }
  .card h3 {
    margin: 0;
    font-size: var(--text-base);
  }
  .plain {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    font-size: var(--text-sm);
    line-height: 1.5;
  }
  .plain a {
    color: var(--link);
  }
  .basis {
    display: block;
    color: var(--text-subtle);
    font-size: var(--text-xs);
    line-height: 1.45;
  }
  .field {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    font-size: var(--text-sm);
  }
  .field.tall {
    flex-direction: column;
    align-items: stretch;
  }
  .field input,
  .bounds input {
    width: 6.5rem;
    box-sizing: border-box;
    min-height: 2.5rem;
    padding: 0 var(--space-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--bg);
    font-variant-numeric: tabular-nums;
  }
  .seg,
  .levels {
    display: flex;
    gap: var(--space-2);
    flex-wrap: wrap;
  }
  .seg button,
  .levels button {
    min-height: 2.5rem;
    padding: 0 var(--space-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-pill);
    background: none;
    cursor: pointer;
  }
  .seg button.on,
  .levels button.on {
    border-color: var(--accent);
    background: var(--accent-subtle);
    font-weight: 600;
  }
  .estimate {
    margin: 0;
    font-size: var(--text-sm);
  }
  .check {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding-top: var(--space-2);
    border-top: 1px solid color-mix(in srgb, var(--border) 60%, transparent);
  }
  .what strong {
    font-size: var(--text-sm);
  }
  .bounds {
    display: flex;
    align-items: flex-end;
    gap: var(--space-3);
    flex-wrap: wrap;
  }
  .bounds label {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: var(--text-xs);
    color: var(--text-muted);
  }
  .unit {
    padding-bottom: 0.65rem;
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .line {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-3);
  }
  .ask,
  .verdict {
    color: var(--text-muted);
    font-size: var(--text-sm);
    text-align: right;
  }
  .value {
    font-size: var(--text-lg);
    font-weight: 650;
    font-variant-numeric: tabular-nums;
  }
  .value small {
    font-size: var(--text-xs);
    font-weight: 400;
    color: var(--text-subtle);
  }
  .bar {
    height: 0.5rem;
    border-radius: var(--radius-pill);
    background: var(--surface-hover);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    border-radius: inherit;
    background: var(--tint-a);
    transition: width var(--dur-med) var(--ease);
  }
  /* Met is the one colour that says "done"; over is said in words and in no warning colour, as on
     the day screen (`decisions.md#ui`, 2026-10-11: going over is not an alarm). */
  .row[data-state='met'] .fill {
    background: var(--tint-b);
  }
  .row[data-state='met'] .verdict {
    color: var(--ok-fg);
  }
  .days {
    margin: 0;
    padding: 0;
    list-style: none;
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 4px;
  }
  .days li {
    display: grid;
    place-items: center;
    height: 1.5rem;
    border-radius: var(--radius-sm);
    background: var(--surface-hover);
    color: var(--text-subtle);
    font-size: var(--text-xs);
  }
  .days li.has {
    background: color-mix(in srgb, var(--tint-b) 35%, transparent);
    color: var(--text);
  }
  .days li.today {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }
  .quiet,
  .go {
    min-height: 2.75rem;
    padding: 0 var(--space-4);
    border-radius: var(--radius-pill);
    cursor: pointer;
  }
  .quiet {
    border: 1px solid var(--border-strong);
    background: none;
  }
  .quiet[aria-pressed='true'] {
    border-color: var(--tint-b);
    background: color-mix(in srgb, var(--tint-b) 20%, transparent);
  }
  .go {
    border: 0;
    background: var(--accent);
    color: var(--accent-contrast);
    font-weight: 600;
  }
  .go:disabled,
  .quiet:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
</style>
