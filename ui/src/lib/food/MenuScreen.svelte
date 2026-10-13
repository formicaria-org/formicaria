<script lang="ts">
  // The days ahead, filled: what is planned for each meal of each day, what the dishes cooked for
  // it will make, and what it all needs bought (`decisions.md#ui`, 2026-10-12, *a menu is the
  // days ahead, filled*).
  //
  // **It writes nothing of its own kind.** A meal on the menu is a planned entry — the same one
  // the day screen shows and confirms — put there with the same sheet. The shopping list is an
  // ordinary note with a box to tick on each line.
  import { entryAmount, shownAmount, shownAs, type AmountUnit } from './amount';
  import Icon from '../Icon.svelte';
  import { logDays, logKitchen, logRecipes, planProgress, recipeLibrary } from '../ipc';
  import { dayAgainst, gap } from '../plan/measure';
  import { createNotesUndoable, deleteNotesUndoable } from '../undo.svelte';
  import { addDays, dayLabel, show, showWith, whole } from '../units';
  import type {
    CarriedRecipe,
    DayTotals,
    NoteDraft,
    ObjectMeta,
    PlanProgress,
    Recipe,
    TotalsDef,
  } from '../types';

  interface Props {
    view: string;
    def: TotalsDef;
    today: string;
    /** Changes whenever something was written: ask again. */
    signal?: unknown;
    onadd: (day: string, slot: string) => void;
    onedit: (entry: ObjectMeta, day: string) => void;
    onsaved: () => void;
  }
  let { view, def, today, signal, onadd, onedit, onsaved }: Props = $props();

  /// The Monday of the week holding a day, and how many days one day is after another.
  const daysApart = (from: string, to: string) =>
    Math.round((Date.parse(`${to}T12:00:00`) - Date.parse(`${from}T12:00:00`)) / 86_400_000);
  const mondayOf = (day: string) => addDays(day, -((new Date(`${day}T12:00:00`).getDay() + 6) % 7));

  const KEY = 'fm-menu-days';
  function stored(): number {
    try {
      const n = Number(localStorage.getItem(KEY));
      return Number.isInteger(n) && n >= 1 && n <= 28 ? n : 7;
    } catch {
      return 7;
    }
  }
  let span = $state(stored());
  function resize(by: number) {
    span = Math.max(1, Math.min(28, span + by));
    try {
      localStorage.setItem(KEY, String(span));
    } catch {
      // A device that keeps nothing still plans; it starts from a week next time.
    }
  }

  let days = $state<DayTotals[]>([]);
  let recipes = $state<Recipe[]>([]);
  /// What each day would come to against the diet followed, counting what is planned.
  let outlook = $state<Record<string, PlanProgress | null>>({});
  /// What the person says they have at home: names, no amounts
  /// (`decisions.md#data`, 2026-10-12, *the kitchen is a list of names*).
  let kitchen = $state<{ id: string; title: string }[]>([]);
  const same = (a: string, b: string) => a.trim().toLowerCase() === b.trim().toLowerCase();
  const had = (title: string) => kitchen.find((k) => same(k.title, title));
  let adding = $state('');
  async function keep(title: string) {
    const name = title.trim();
    if (!name || had(name)) return;
    try {
      await createNotesUndoable([{ title: name, props: { kitchen: 1 } }], `have “${name}”`);
      adding = '';
      onsaved();
      await load();
    } catch (e) {
      failed = String(e);
    }
  }
  async function out(item: { id: string; title: string }) {
    try {
      await deleteNotesUndoable([item.id], `ran out of “${item.title}”`);
      onsaved();
      await load();
    } catch (e) {
      failed = String(e);
    }
  }
  let failed = $state('');
  let said = $state('');
  let asked = 0;
  async function load() {
    const mine = ++asked;
    try {
      const [d, r, k] = await Promise.all([
        logDays(view, today, addDays(today, span - 1), true),
        logRecipes(view),
        logKitchen(),
      ]);
      if (mine !== asked) return;
      days = d.days;
      recipes = r;
      kitchen = k;
      // Each day held against the diet as if what is planned had been had. **One ask a week**,
      // not one a day: an answer carries its whole week, Monday to Sunday, a value a day — so a
      // menu of 28 days is five asks. Never why the menu fails to draw.
      const mondays = [...new Set(d.days.map((x) => mondayOf(x.day)))];
      const weeks = await Promise.all(
        mondays.map((m) => planProgress('food', m, true).catch(() => null)),
      );
      if (mine !== asked) return;
      const out: Record<string, PlanProgress | null> = {};
      for (const x of d.days) {
        const week = weeks[mondays.indexOf(mondayOf(x.day))];
        const at = daysApart(mondayOf(x.day), x.day);
        // The week's answer, read for this one day: each daily check's value that day, and
        // whether it is within what the note holds the person to.
        out[x.day] =
          week?.plan && (x.rows ?? []).length
            ? {
                ...week,
                checks: week.checks
                  .filter((c) => c.per === 'day')
                  .map((c) => {
                    const v = c.days[at];
                    const value = v === null || v === undefined ? undefined : v;
                    const state =
                      value === undefined
                        ? ('none' as const)
                        : c.max !== undefined && value > c.max
                          ? ('over' as const)
                          : c.min !== undefined && value < c.min
                            ? ('under' as const)
                            : ('met' as const);
                    return { ...c, value, state };
                  }),
              }
            : null;
      }
      outlook = out;
      failed = '';
    } catch (e) {
      if (mine === asked) failed = String(e);
    }
  }
  $effect(() => {
    void signal;
    void span;
    void load();
  });

  const primary = $derived(def.metrics.find((m) => m.role === 'primary') ?? def.metrics[0]);
  const rowsOf = (d: DayTotals, slot: string) =>
    (d.rows ?? []).filter((r) => String(r.props.slot ?? '') === slot);
  /// What a day would come to: what was had and what is planned, together.
  const cameTo = (d: DayTotals) =>
    (d.rows ?? []).reduce((t, r) => t + (whole(r.props[primary.key]) ?? 0), 0);
  const amount = (r: ObjectMeta) => {
    const pct = whole(r.props.servings_pct);
    if (pct !== null) return pct === 100 ? '1 serving' : `${pct / 100} servings`;
    return shownAmount(r.props);
  };
  const wordOf = (r: ObjectMeta) => {
    const s = String(r.props.source ?? '');
    return s.startsWith('recipe/') ? s.slice('recipe/'.length) : '';
  };

  /// The dishes on this menu and how many of the servings each makes are spoken for.
  const dishes = $derived.by(() => {
    const used = new Map<string, number>();
    for (const d of days)
      for (const r of d.rows ?? []) {
        const w = wordOf(r);
        if (w) used.set(w, (used.get(w) ?? 0) + (whole(r.props.servings_pct) ?? 100));
      }
    return [...used.entries()].flatMap(([word, pct]) => {
      const recipe = recipes.find((x) => x.recipe === word);
      return recipe ? [{ recipe, servings: pct / 100 }] : [];
    });
  });

  /// The same meal, the day after: a dish cooked once is eaten again.
  async function again(r: ObjectMeta, day: string) {
    const { planned: _planned, ...keep } = r.props;
    const props = Object.fromEntries(
      Object.entries(keep).filter(
        (e): e is [string, string | number] => typeof e[1] === 'string' || typeof e[1] === 'number',
      ),
    );
    try {
      await createNotesUndoable(
        [{ title: r.title ?? '', start: addDays(day, 1), props: { ...props, planned: 1 } }],
        `planned “${r.title ?? 'a meal'}” again`,
      );
      onsaved();
      await load();
    } catch (e) {
      failed = String(e);
    }
  }

  /// What the planned meals need: a dish's ingredients for the servings planned, a plain food by
  /// its amount; the same thing added up. Only what is still planned — what was eaten was had.
  const needs = $derived.by(() => {
    // Grams and millilitres are added up apart: a drink is not turned into a weight.
    const sum = new Map<string, { title: string; unit: AmountUnit; n: number }>();
    const add = (title: string, props: Record<string, unknown>, part = 1) => {
      const a = entryAmount(props);
      const n = (a.n ?? 0) * part;
      if (n <= 0) return;
      const key = `${title}\u0000${a.unit}`;
      sum.set(key, { title, unit: a.unit, n: (sum.get(key)?.n ?? 0) + n });
    };
    for (const d of days)
      for (const r of d.rows ?? []) {
        if (!r.props.planned) continue;
        const recipe = recipes.find((x) => x.recipe === wordOf(r));
        if (recipe) {
          const part = (whole(r.props.servings_pct) ?? 100) / 100 / Math.max(1, recipe.servings);
          for (const i of recipe.ingredients) add(i.title ?? 'Untitled', i.props, part);
        } else add(r.title ?? 'Untitled', r.props);
      }
    return [...sum.values()]
      .map((x) => ({ title: x.title, unit: x.unit, grams: Math.round(x.n) }))
      .sort((a, b) => a.title.localeCompare(b.title));
  });

  /// What is needed and not at home.
  const toBuy = $derived(needs.filter((n) => !had(n.title)));

  /// Planned dishes from the recipes carried in the app: their ingredient lines as published —
  /// in cups and spoons, tied to no food, so they are listed and not added up.
  let library = $state<CarriedRecipe[]>([]);
  $effect(() => {
    recipeLibrary()
      .then((r) => (library = r.recipes))
      .catch(() => (library = []));
  });
  const asPublished = $derived.by(() => {
    const ids = new Set<string>();
    for (const d of days)
      for (const r of d.rows ?? []) {
        const s = String(r.props.source ?? '');
        if (r.props.planned && s.startsWith('nhlbi/')) ids.add(s.slice('nhlbi/'.length));
      }
    return library.filter((r) => ids.has(r.id));
  });

  let busy = $state(false);
  async function shoppingList() {
    if (busy || (!toBuy.length && !asPublished.length)) return;
    busy = true;
    const from = dayLabel(today);
    const to = dayLabel(addDays(today, span - 1));
    const draft: NoteDraft = {
      title: `Shopping list, ${from} to ${to}`,
      body:
        `What the menu from ${from} to ${to} needs, added up from the planned meals and the ` +
        `recipes behind them. Tick what you have or have bought.\n\n` +
        toBuy.map((n) => `- [ ] ${n.title} — ${shownAs(n.grams, n.unit)}`).join('\n') +
        (needs.length > toBuy.length
          ? `\n\nLeft off because you have them: ${needs
              .filter((n) => had(n.title))
              .map((n) => n.title)
              .join(', ')}.`
          : '') +
        asPublished
          .map(
            (r) =>
              `\n\nFor ${r.title} (${r.yield ?? 'as published'}), as its source lists them:\n\n` +
              r.ingredients.map((line) => `- [ ] ${line}`).join('\n'),
          )
          .join('') +
        '\n',
      props: {},
    };
    try {
      await createNotesUndoable([draft], 'made a shopping list');
      said = `“${draft.title}” is in your notes.`;
      onsaved();
    } catch (e) {
      failed = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="menu">
  <header class="head">
    <h2>Menu</h2>
    <div class="span" role="group" aria-label="how many days">
      <button type="button" class="round" onclick={() => resize(-1)} aria-label="one day fewer">
        <Icon name="minus" size={18} />
      </button>
      <span>{span} {span === 1 ? 'day' : 'days'}</span>
      <button type="button" class="round" onclick={() => resize(1)} aria-label="one day more">
        <Icon name="plus" size={18} />
      </button>
    </div>
  </header>
  {#if failed}<p class="failed" role="alert">{failed}</p>{/if}

  {#each days as d (d.day)}
    {@const total = cameTo(d)}
    {@const target = d.targets[primary.key] ?? 0}
    <section class="day" aria-label={dayLabel(d.day)}>
      <header>
        <h3>{d.day === today ? 'Today' : dayLabel(d.day)}</h3>
        <span class="sum">
          {#if total > 0}
            {show(primary.key, total).text}{#if target > 0}&nbsp;of {showWith(
                primary.key,
                target,
              )}{:else}&nbsp;{show(primary.key, total).unit}{/if}
          {:else}
            nothing yet
          {/if}
        </span>
      </header>
      {#if outlook[d.day]?.plan}
        {@const against = dayAgainst(outlook[d.day]!.checks)}
        {#if against.daily && !against.nothing}
          <p class="against">
            As planned: {against.met} of {against.daily} of what {outlook[d.day]!.plan!.name} asks of
            a day{against.off.length ? ' — ' : '.'}{against.off
              .map((c) => `${c.label.toLowerCase()} ${gap(c)}`)
              .join(', ')}
          </p>
        {/if}
      {/if}
      {#each def.slots as s (s.value)}
        {@const list = rowsOf(d, s.value)}
        <div class="slot">
          <span class="label">{s.label}</span>
          <ul>
            {#each list as r (r.id)}
              <li class:had={!r.props.planned}>
                <button
                  type="button"
                  class="entry"
                  onclick={() => onedit(r, d.day)}
                  aria-label={`change ${r.title ?? 'a meal'}`}
                >
                  <span class="title">{r.title ?? 'Untitled'}</span>
                  <span class="amount">{amount(r)}</span>
                </button>
                <button
                  type="button"
                  class="next"
                  onclick={() => again(r, d.day)}
                  aria-label={`${r.title ?? 'this'} again the day after`}
                  title="Again the day after"
                >
                  <Icon name="plus" size={14} /> next day
                </button>
              </li>
            {/each}
          </ul>
          <button
            type="button"
            class="add"
            onclick={() => onadd(d.day, s.value)}
            aria-label={`plan ${s.label} for ${dayLabel(d.day)}`}
          >
            <Icon name="plus" size={18} />
          </button>
        </div>
      {/each}
    </section>
  {/each}

  {#if dishes.length}
    <section class="card">
      <h3>Dishes on this menu</h3>
      <ul class="plain">
        {#each dishes as x (x.recipe.recipe)}
          <li>
            <span>{x.recipe.title}</span>
            <span class:short={x.servings > x.recipe.servings}>
              {x.servings} of the {x.recipe.servings} it makes{x.servings > x.recipe.servings
                ? ' — more than one batch'
                : ''}
            </span>
          </li>
        {/each}
      </ul>
      <p class="says">
        How long a cooked dish keeps is not something this app can tell you: no source for it has
        been read.
      </p>
    </section>
  {/if}

  <section class="card">
    <h3>What it needs</h3>
    {#if needs.length || asPublished.length}
      <ul class="plain needs">
        {#each needs as n (n.title + n.unit)}
          {@const mine = had(n.title)}
          <li class:have={!!mine}>
            <span>{n.title}</span>
            <span>{shownAs(n.grams, n.unit)}</span>
            <button
              type="button"
              class="have-it"
              aria-pressed={!!mine}
              onclick={() => (mine ? out(mine) : keep(n.title))}
              aria-label={mine ? `no longer have ${n.title}` : `have ${n.title}`}
            >
              {mine ? 'have it' : 'need it'}
            </button>
          </li>
        {/each}
      </ul>
      {#each asPublished as r (r.id)}
        <p class="says">For {r.title} ({r.yield ?? 'as published'}), as its source lists them:</p>
        <ul class="plain">
          {#each r.ingredients as line, i (i)}<li><span>{line}</span></li>{/each}
        </ul>
      {/each}
      <button
        type="button"
        class="go"
        onclick={shoppingList}
        disabled={busy || (!toBuy.length && !asPublished.length)}
      >
        Make a shopping list{toBuy.length < needs.length ? ` of ${toBuy.length}` : ''}
      </button>
      {#if said}<p class="says" role="status">{said}</p>{/if}
    {:else}
      <p class="says">
        Plan a meal with the plus beside it. A dish from your recipes brings its ingredients here.
      </p>
    {/if}
  </section>

  <section class="card" aria-label="in your kitchen">
    <h3>In your kitchen</h3>
    {#if kitchen.length}
      <ul class="chips">
        {#each kitchen as k (k.id)}
          <li>
            <span>{k.title}</span>
            <button type="button" onclick={() => out(k)} aria-label={`no longer have ${k.title}`}>
              <Icon name="close" size={14} />
            </button>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="says">
        What you say you have is left off the shopping list. Names only: no amounts to keep up.
      </p>
    {/if}
    <form
      class="addrow"
      onsubmit={(e) => {
        e.preventDefault();
        void keep(adding);
      }}
    >
      <input
        type="text"
        bind:value={adding}
        placeholder="Something you have"
        aria-label="something you have"
        autocomplete="off"
      />
      <button type="submit" class="quiet" disabled={!adding.trim()}>Add</button>
    </form>
  </section>
</div>

<style>
  .menu {
    height: 100%;
    overflow-y: auto;
    overscroll-behavior: contain;
    box-sizing: border-box;
    padding: var(--space-3) max(var(--space-3), calc((100% - 36rem) / 2)) var(--space-8);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    scrollbar-width: thin;
  }
  .menu > :global(*) {
    flex-shrink: 0;
  }
  .head,
  .day > header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }
  h2 {
    margin: 0;
    font-size: var(--text-xl);
  }
  h3 {
    margin: 0;
    font-size: var(--text-md);
  }
  button {
    font: inherit;
    color: inherit;
  }
  .span {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-variant-numeric: tabular-nums;
  }
  .round,
  .add {
    flex: none;
    width: 2.75rem;
    height: 2.75rem;
    display: grid;
    place-items: center;
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    background: none;
    cursor: pointer;
  }
  .add {
    color: var(--accent);
  }
  .day,
  .card {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }
  .against {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .sum,
  .says,
  .label,
  .amount {
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .says {
    margin: 0;
  }
  /* The meal's name and its plus on one line, what is planned under them at full width: a
     title has the whole row on a phone. */
  .slot {
    display: grid;
    grid-template-columns: 1fr auto;
    grid-template-areas:
      'label add'
      'list list';
    align-items: center;
    gap: var(--space-1) var(--space-2);
    border-top: 1px solid var(--border);
    padding-top: var(--space-2);
  }
  .slot .label {
    grid-area: label;
  }
  .slot .add {
    grid-area: add;
  }
  .slot ul {
    grid-area: list;
  }
  .slot ul:empty {
    display: none;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  .slot li {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    min-width: 0;
  }
  .slot li.had .title {
    color: var(--text-muted);
  }
  .entry {
    flex: 1;
    min-width: 0;
    min-height: 2.75rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    padding: 0 var(--space-2);
    border: 0;
    border-radius: var(--radius-md);
    background: var(--surface-hover);
    text-align: left;
    cursor: pointer;
  }
  .entry .amount {
    flex: none;
  }
  .entry .title {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .next {
    flex: none;
    min-height: 2.5rem;
    padding: 0 var(--space-2);
    border: 0;
    background: none;
    font-size: var(--text-xs);
    color: var(--text-muted);
    cursor: pointer;
    white-space: nowrap;
  }
  .plain li {
    display: flex;
    justify-content: space-between;
    gap: var(--space-2);
    font-size: var(--text-sm);
  }
  .short {
    color: var(--danger-fg);
  }
  .go {
    align-self: flex-start;
    min-height: 2.75rem;
    padding: 0 var(--space-4);
    border: 0;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--accent-contrast);
    font-weight: 600;
    cursor: pointer;
  }
  .go:disabled {
    opacity: 0.5;
  }
  .failed {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--danger-fg);
  }
  .needs li {
    display: grid;
    grid-template-columns: 1fr auto auto;
    align-items: center;
  }
  .needs li.have span {
    color: var(--text-muted);
    text-decoration: line-through;
  }
  .have-it {
    min-height: 2.25rem;
    min-width: 4.5rem;
    padding: 0 var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    background: none;
    font-size: var(--text-xs);
    cursor: pointer;
  }
  .have-it[aria-pressed='true'] {
    border-color: var(--accent);
    background: var(--accent-subtle);
  }
  .chips {
    flex-direction: row;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  .chips li {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    padding-left: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    font-size: var(--text-sm);
  }
  .chips button {
    width: 2.25rem;
    height: 2.25rem;
    display: grid;
    place-items: center;
    border: 0;
    background: none;
    color: var(--text-muted);
    cursor: pointer;
  }
  .addrow {
    display: flex;
    gap: var(--space-2);
  }
  .addrow input {
    flex: 1;
    min-width: 0;
    min-height: 2.75rem;
    padding: 0 var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: inherit;
    font: inherit;
  }
  .quiet {
    min-height: 2.75rem;
    padding: 0 var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: none;
    cursor: pointer;
  }
  .quiet:disabled {
    opacity: 0.5;
  }
</style>
