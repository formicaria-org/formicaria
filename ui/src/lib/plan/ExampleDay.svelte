<script lang="ts">
  // A day of eating to start from (`decisions.md#data`, 2026-10-12, *every regime has a week to
  // start from*, which holds for diets as days). Either a body's own sample day, **as published
  // and unchanged**, or three of the recipes the app carries, **chosen by arithmetic** — and then
  // the screen says what the day meets and what it falls short of, instead of padding it out.
  // Only an indication. A day of recipes can be planned for today, to change.
  import { recipeLibrary } from '../ipc';
  import { createNotesUndoable } from '../undo.svelte';
  import type { CarriedRecipe, NoteDraft, PlanFoodExample } from '../types';

  interface Props {
    example: PlanFoodExample;
    today: string;
    onsaved: () => void;
  }
  let { example, today, onsaved }: Props = $props();

  // The example days name recipes of the first carried collection — the one with figures.
  let recipes = $state<CarriedRecipe[]>([]);
  let key = $state('');
  $effect(() => {
    if (!example.recipes) return;
    void recipeLibrary()
      .then((l) => {
        recipes = l.recipes;
        key = l.key;
      })
      .catch(() => (recipes = []));
  });
  const titleOf = (id: string) => recipes.find((r) => r.id === id)?.title ?? id;
  /// The meal a part of the day belongs to, as the day screen names its parts.
  const slotOf = (meal: string) =>
    /breakfast/i.test(meal)
      ? 'breakfast'
      : /dinner|supper|evening/i.test(meal)
        ? 'dinner'
        : 'lunch';
  const TOTALS: [string, string, string][] = [
    ['kcal', 'Energy', 'kcal'],
    ['fat_pct', 'Fat', '% of energy'],
    ['saturated_fat_pct', 'Saturated fat', '% of energy'],
    ['carbs_pct', 'Carbohydrate', '% of energy'],
    ['protein_pct', 'Protein', '% of energy'],
    ['fiber_g', 'Fibre', 'g'],
    ['sodium_mg', 'Sodium', 'mg'],
    ['potassium_mg', 'Potassium', 'mg'],
  ];
  let busy = $state(false);
  let said = $state('');
  let failed = $state('');
  async function plan() {
    if (busy || !example.recipes) return;
    busy = true;
    failed = '';
    try {
      const drafts: NoteDraft[] = example.recipes.flatMap((x) => {
        const r = recipes.find((y) => y.id === x.recipe);
        return r
          ? [
              {
                title: r.title,
                start: today,
                props: {
                  log: 'food',
                  slot: slotOf(x.meal),
                  planned: 1,
                  source: `${key}/${r.id}`,
                  ...r.per_serving,
                },
              },
            ]
          : [];
      });
      if (!drafts.length) throw new Error('the recipes could not be read');
      await createNotesUndoable(drafts, 'planned an example day');
      said = 'Planned for today: confirm each on the Day tab when you eat it, or change it.';
      onsaved();
    } catch (e) {
      failed = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<section class="example" aria-label="a day to start from">
  <h3>
    A day to start from{example.kind === 'published' ? ', as published' : ', from the recipes here'}
  </h3>
  <p class="note">{example.note}</p>
  {#if example.day}
    <!-- The body's own day, word for word. -->
    <div class="theirs">
      {#each example.day.meals as m (m.meal)}
        <h4>{m.meal}</h4>
        <ul>
          {#each m.items as item, i (i)}<li>{item}</li>{/each}
        </ul>
      {/each}
      <p class="note">
        {Object.entries(example.day.totals ?? {})
          .map(([k, v]) => `${k}: ${v}`)
          .join(' · ')}
      </p>
    </div>
    {#if example.fits}<p class="note">{example.fits}</p>{/if}
  {:else if example.recipes}
    <ul>
      {#each example.recipes as x (x.meal + x.recipe)}
        <li><span class="meal">{x.meal}</span> {titleOf(x.recipe)}</li>
      {/each}
    </ul>
    <p class="note">
      {TOTALS.filter(([k]) => example.totals?.[k] !== undefined)
        .map(([k, label, unit]) => `${label} ${example.totals?.[k]} ${unit}`)
        .join(' · ')}
    </p>
    {#if example.against}<p class="note">{example.against}</p>{/if}
  {/if}
  <p class="note">
    {example.kind === 'published' ? 'Published by' : 'Chosen by'}
    {example.by} ·
    <a href={example.url} target="_blank" rel="noreferrer noopener">{example.title}</a>
  </p>
  {#if failed}<p class="failed" role="alert">{failed}</p>{/if}
  {#if example.recipes}
    {#if said}
      <p class="note" role="status">{said}</p>
    {:else}
      <button type="button" class="go" onclick={plan} disabled={busy || !recipes.length}>
        Plan it for today
      </button>
    {/if}
  {/if}
</section>

<style>
  a {
    color: var(--accent);
  }
  .example {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }
  h3 {
    margin: 0;
    font-size: var(--text-md);
  }
  h4 {
    margin: var(--space-1) 0 0;
    font-size: var(--text-sm);
  }
  .note {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  ul {
    margin: 0;
    padding-left: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: var(--text-sm);
  }
  .theirs {
    padding-left: var(--space-3);
    border-left: 3px solid var(--border);
  }
  .meal {
    color: var(--text-muted);
  }
  .go {
    align-self: flex-start;
    min-height: 2.75rem;
    padding: 0 var(--space-4);
    border: 0;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--accent-contrast);
    font: inherit;
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
