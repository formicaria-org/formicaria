<script lang="ts">
  // The days ahead, filled: what is planned for each meal of each day, what the dishes cooked for
  // it will make, and what it all needs bought (`decisions.md#ui`, 2026-10-12, *a menu is the
  // days ahead, filled*).
  //
  // **It writes nothing of its own kind.** A meal on the menu is a planned entry — the same one
  // the day screen shows and confirms — put there with the same sheet. The shopping list is an
  // ordinary note with a box to tick on each line.
  import Icon from '../Icon.svelte';
  import { logDays, logRecipes } from '../ipc';
  import { createNotesUndoable } from '../undo.svelte';
  import { addDays, dayLabel, show, showWith, whole } from '../units';
  import type { DayTotals, NoteDraft, ObjectMeta, Recipe, TotalsDef } from '../types';

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
  let failed = $state('');
  let said = $state('');
  let asked = 0;
  async function load() {
    const mine = ++asked;
    try {
      const [d, r] = await Promise.all([
        logDays(view, today, addDays(today, span - 1), true),
        logRecipes(view),
      ]);
      if (mine !== asked) return;
      days = d.days;
      recipes = r;
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
    const g = whole(r.props.amount_g);
    return g !== null ? showWith('amount_g', g) : '';
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
    const sum = new Map<string, number>();
    const add = (title: string, grams: number) =>
      grams > 0 && sum.set(title, (sum.get(title) ?? 0) + grams);
    for (const d of days)
      for (const r of d.rows ?? []) {
        if (!r.props.planned) continue;
        const recipe = recipes.find((x) => x.recipe === wordOf(r));
        if (recipe) {
          const part = (whole(r.props.servings_pct) ?? 100) / 100 / Math.max(1, recipe.servings);
          for (const i of recipe.ingredients)
            add(i.title ?? 'Untitled', (whole(i.props.amount_g) ?? 0) * part);
        } else add(r.title ?? 'Untitled', whole(r.props.amount_g) ?? 0);
      }
    return [...sum.entries()]
      .map(([title, grams]) => ({ title, grams: Math.round(grams) }))
      .sort((a, b) => a.title.localeCompare(b.title));
  });

  let busy = $state(false);
  async function shoppingList() {
    if (busy || !needs.length) return;
    busy = true;
    const from = dayLabel(today);
    const to = dayLabel(addDays(today, span - 1));
    const draft: NoteDraft = {
      title: `Shopping list, ${from} to ${to}`,
      body:
        `What the menu from ${from} to ${to} needs, added up from the planned meals and the ` +
        `recipes behind them. Tick what you have or have bought.\n\n` +
        needs.map((n) => `- [ ] ${n.title} — ${showWith('amount_g', n.grams)}`).join('\n') +
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
    {#if needs.length}
      <ul class="plain">
        {#each needs as n (n.title)}
          <li><span>{n.title}</span><span>{showWith('amount_g', n.grams)}</span></li>
        {/each}
      </ul>
      <button type="button" class="go" onclick={shoppingList} disabled={busy}>
        Make a shopping list
      </button>
      {#if said}<p class="says" role="status">{said}</p>{/if}
    {:else}
      <p class="says">
        Plan a meal with the plus beside it. A dish from your recipes brings its ingredients here.
      </p>
    {/if}
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
</style>
