<script lang="ts">
  // Logging a food, or changing one already logged.
  //
  // **This is a form about food, on purpose** — one of the closed set of domain parts a group's add
  // control may open (`decisions.md#ui`, 2026-10-11). A sheet with one number field per metric
  // would be a spreadsheet row. So it opens on what you ate before, which needs no keyboard at all;
  // search and typing a food in by hand are one tap further.
  //
  // The words for meals and values still come from the view's definition, and the units from the
  // keys. An amount is never rescaled from numbers that were already rounded: it is recomputed from
  // where the food came from, or from the values as they were when this sheet opened.
  import { onMount, untrack } from 'svelte';
  import Icon from '../Icon.svelte';
  import {
    logRecent,
    logRecipes,
    logProducts,
    productLookup,
    recipeLibrary,
    foodSearch,
    foodGet,
  } from '../ipc';
  import { digitsOf, isBarcode, sameCode } from './barcode';
  import {
    createNotesUndoable,
    setPropertiesUndoable,
    deleteUndoable,
    deleteNotesUndoable,
  } from '../undo.svelte';
  import { show, showWith, today, typed, whole, nowClock, clockOf, dayOf } from '../units';
  import { RECIPE, recipeWord } from '../templates';
  import type {
    CarriedRecipe,
    FoodHit,
    FoodPortion,
    Metric,
    NoteDraft,
    ObjectMeta,
    Product,
    RecentItem,
    Recipe,
    RecipeLibrary,
    TotalsDef,
  } from '../types';

  interface Props {
    view: string;
    def: TotalsDef;
    day: string;
    /** The slot the add control was pressed on, or `null` to go by the clock. */
    slot: string | null;
    /** An entry to change, instead of adding one. */
    entry?: ObjectMeta | null;
    onclose: () => void;
    /** Something was written; `removed` names an entry that was taken away. */
    onwrote: (what?: { removed?: string }) => void;
  }
  let { view, def, day, slot, entry = null, onclose, onwrote }: Props = $props();

  const AMOUNT = 'amount_g';
  /// The food tables the list is made from, as a person would name them
  /// (`decisions.md#data`, 2026-10-11).
  const TABLES: Record<string, string> = { ciqual: 'Ciqual', bls: 'BLS', usda: 'USDA' };
  const primary = $derived(def.metrics.find((m) => m.role === 'primary') ?? def.metrics[0]);
  const secondary = $derived(def.metrics.filter((m) => m.role === 'secondary'));

  // With no slot given, none is guessed. Which meal something belongs to is the person's to say:
  // the labels are the ones a person picks from in dietary surveys, and no agreed rule turns a
  // time of day into one of them (`docs/context/sources-2026-10-11.md`, *Meal occasions*).
  // Read once, on purpose: the sheet is opened for one slot or one entry and closed again.
  let chosen = $state(untrack(() => (entry ? String(entry.props.slot ?? '') : (slot ?? ''))));
  /// An amount known only roughly: the least and the most it could be, in grams
  /// (`decisions.md#data`, 2026-10-12, *an amount may be a range*). Empty until the person asks.
  let unsure = $state(untrack(() => !!entry && whole(entry.props.amount_min_g) !== null));
  let least = $state<number | string | null>(untrack(() => whole(entry?.props.amount_min_g)));
  let most = $state<number | string | null>(untrack(() => whole(entry?.props.amount_max_g)));
  /// The range as it is kept: both ends, holding the amount between them — or nothing.
  function rangeProps(grams: number): Record<string, number | string> {
    const [lo, hi] = [Math.round(Number(least)), Math.round(Number(most))];
    const ok = unsure && lo > 0 && hi > lo && lo <= grams && grams <= hi;
    if (ok) return { amount_min_g: lo, amount_max_g: hi };
    // Changing an entry that had one: an empty value takes the key away.
    return entry && whole(entry.props.amount_min_g) !== null
      ? { amount_min_g: '', amount_max_g: '' }
      : {};
  }
  /// The time of an entry being changed, `HH:MM`, or empty when it has none.
  let at = $state(untrack(() => clockOf(entry?.start)));
  const slotLabel = $derived(def.slots.find((s) => s.value === chosen)?.label ?? '');
  const unplaced = $derived(def.slots.length > 0 && !chosen);

  type Mode = 'recent' | 'search' | 'recipes' | 'quick';
  let mode = $state<Mode>('recent');
  let failed = $state('');
  let busy = $state(false);
  let added = $state(0);

  // ── what was eaten before ──────────────────────────────────────────────────────────────────────
  let recent = $state<RecentItem[] | null>(null);
  $effect(() => {
    if (entry) return;
    const s = chosen;
    logRecent(view, { ...(s ? { slot: s } : {}), limit: 20 })
      .then((r) => (recent = r))
      .catch((e) => (failed = String(e)));
  });

  // ── search ─────────────────────────────────────────────────────────────────────────────────────
  let q = $state('');
  let hits = $state<FoodHit[]>([]);
  $effect(() => {
    const text = q.trim();
    if (!text) {
      hits = [];
      return;
    }
    const t = setTimeout(() => {
      foodSearch(text)
        .then((r) => (hits = r))
        .catch((e) => (failed = String(e)));
    }, 120);
    return () => clearTimeout(t);
  });

  // ── packaged foods, from their labels ──────────────────────────────────────────────────────────
  // A packet is not in a table of generic foods. It is written down once from its label — per
  // 100 g, in the order a European label prints it — and is one of the person's foods from then
  // on (`decisions.md#data`, 2026-10-12, *a packaged food is a note read off its label*).
  let products = $state<Product[]>([]);
  const loadProducts = () =>
    logProducts()
      .then((r) => (products = r))
      .catch(() => (products = []));
  onMount(loadProducts);
  /// The person's own packaged foods that the search text names, by name or by barcode.
  const mine = $derived.by(() => {
    const text = q.trim().toLowerCase();
    if (!text) return products;
    const code = sameCode(text);
    return products.filter(
      (p) => p.title.toLowerCase().includes(text) || (!!p.barcode && sameCode(p.barcode) === code),
    );
  });
  const LABEL: { key: string; label: string; per: number }[] = [
    { key: 'kcal', label: 'Energy (kcal)', per: 1 },
    { key: 'fat_mg', label: 'Fat (g)', per: 1000 },
    { key: 'saturated_fat_mg', label: 'of which saturates (g)', per: 1000 },
    { key: 'carbs_mg', label: 'Carbohydrate (g)', per: 1000 },
    { key: 'sugars_mg', label: 'of which sugars (g)', per: 1000 },
    { key: 'fiber_mg', label: 'Fibre (g)', per: 1000 },
    { key: 'protein_mg', label: 'Protein (g)', per: 1000 },
    // A label states salt; the tables and every reference here are in sodium. Salt is sodium
    // × 2.5 (Regulation (EU) 1169/2011, Annex I), so a gram of salt is 400 mg of sodium.
    { key: 'sodium_ug', label: 'Salt (g)', per: 400_000 },
  ];
  let labelling = $state<{
    name: string;
    barcode: string;
    serving: number | string | null;
    ingredients: string;
    values: Record<string, number | string | null | undefined>;
    /** Where the figures were looked up, when they were. */
    page: string;
  } | null>(null);
  /// Asking Open Food Facts for the product under a barcode: one request, on this press only
  /// (`decisions.md#data`, 2026-10-12). What comes back fills the form; the person saves it.
  let looking = $state(false);
  let lookedUp = $state('');
  async function lookUp() {
    const l = labelling;
    if (!l || looking || !isBarcode(l.barcode)) return;
    looking = true;
    lookedUp = '';
    try {
      const found = await productLookup(digitsOf(l.barcode));
      if (labelling !== l) return;
      if (!found) {
        lookedUp = 'Open Food Facts does not have this product. Type its label in below.';
        return;
      }
      l.name = l.name.trim() || found.name;
      l.serving = l.serving || (found.serving_g ?? null);
      l.ingredients = l.ingredients.trim() || found.ingredients;
      l.page = found.page;
      for (const f of LABEL) {
        const v = found.per100[f.key];
        if (v !== undefined) l.values[f.key] = Math.round((v / f.per) * 100) / 100;
      }
      lookedUp =
        found.per100.kcal === undefined
          ? 'Found, but with no energy figure: copy it from the pack.'
          : 'Found. Check it against the pack before you save: volunteers typed it in.';
    } catch (e) {
      lookedUp = String(e).replace(/^Error: /, '');
    } finally {
      looking = false;
    }
  }
  const startLabel = () =>
    (labelling = {
      name: /^[\d\s-]+$/.test(q.trim()) ? '' : q.trim(),
      barcode: /^[\d\s-]+$/.test(q.trim()) ? digitsOf(q) : '',
      serving: null,
      ingredients: '',
      values: {},
      page: '',
    });
  const labelValues = $derived.by(() => {
    const out: Record<string, number> = {};
    for (const f of LABEL) {
      const raw = labelling?.values[f.key];
      if (raw === '' || raw === null || raw === undefined) continue;
      const n = Number(raw);
      if (Number.isFinite(n) && n >= 0) out[f.key] = Math.round(n * f.per);
    }
    return out;
  });
  const badCode = $derived(!!labelling?.barcode.trim() && !isBarcode(labelling.barcode));
  const labelReady = $derived(
    !!labelling?.name.trim() && labelValues.kcal !== undefined && !badCode,
  );
  const saveLabel = () =>
    write(async () => {
      const l = labelling;
      if (!l || !labelReady) return;
      const title = l.name.trim();
      const serving = Math.round(Number(l.serving));
      await createNotesUndoable(
        [
          {
            title,
            body:
              (l.ingredients.trim()
                ? `Ingredients, as on the pack:\n\n${l.ingredients.trim()}\n`
                : '') +
              (l.page
                ? `\nLooked up at [Open Food Facts](${l.page}) and checked by you. Contains data ` +
                  `from Open Food Facts, available under the Open Database License.\n`
                : ''),
            props: {
              product: 'label',
              ...(l.page ? { source_url: l.page } : {}),
              ...(l.barcode.trim() ? { barcode: digitsOf(l.barcode) } : {}),
              ...(serving > 0 ? { serving_g: serving } : {}),
              ...labelValues,
            },
          },
        ],
        `wrote down “${title}” from its label`,
      );
      await loadProducts();
      labelling = null;
      const made = products.find((p) => p.title === title);
      if (made) openProduct(made);
    });
  function openProduct(p: Product) {
    failed = '';
    portion = {
      title: p.title,
      grams: p.serving_g ?? 100,
      per100: p.per100,
      base: {},
      baseGrams: 0,
      portions: p.serving_g ? [{ label: `1 serving (${p.serving_g} g)`, grams: p.serving_g }] : [],
      source: `product/${p.id}`,
    };
  }

  // ── recipes carried in the app ─────────────────────────────────────────────────────────────────
  // The NHLBI's, word for word (`decisions.md#data`, 2026-10-12). Asked for when the Recipes tab
  // is first opened. A serving is logged with the Institute's own values; nothing is worked out.
  let carried = $state<RecipeLibrary | null>(null);
  let reading = $state<CarriedRecipe | null>(null);
  $effect(() => {
    if (mode === 'recipes' && !carried)
      recipeLibrary()
        .then((r) => (carried = r))
        .catch(() => (carried = null));
  });
  const addCarried = () =>
    write(async () => {
      const r = reading;
      if (!r) return;
      await createNotesUndoable(
        [draft(r.title, { ...r.per_serving, source: `nhlbi/${r.id}` })],
        `logged a serving of “${r.title}”`,
      );
      added += 1;
      reading = null;
      onwrote();
    });

  // ── the portion step ───────────────────────────────────────────────────────────────────────────
  type Portion = {
    title: string;
    grams: number;
    /** Values per 100 g, when the food's source is known. */
    per100: Record<string, number> | null;
    /** Otherwise: the values as they were at `baseGrams`, kept so stepping never compounds. */
    base: Record<string, number>;
    baseGrams: number;
    portions: FoodPortion[];
    source: string;
  };
  let portion = $state<Portion | null>(null);

  const numbersOf = (props: Record<string, unknown>) =>
    Object.fromEntries(
      def.metrics.flatMap((m) => {
        const v = whole(props[m.key]);
        return v === null ? [] : [[m.key, v] as const];
      }),
    );
  function valuesAt(p: Portion, grams: number): Record<string, number> {
    if (p.per100)
      return Object.fromEntries(
        Object.entries(p.per100).map(([k, v]) => [k, Math.round((v * grams) / 100)]),
      );
    if (p.baseGrams <= 0) return p.base;
    return Object.fromEntries(
      Object.entries(p.base).map(([k, v]) => [k, Math.round((v * grams) / p.baseGrams)]),
    );
  }
  const now = $derived(portion ? valuesAt(portion, portion.grams) : {});

  async function openPortion(title: string, props: Record<string, unknown>) {
    if (whole(props[PCT]) !== null) return openServing(title, props);
    failed = '';
    const grams = whole(props[AMOUNT]) ?? 0;
    const source = typeof props.source === 'string' ? props.source : '';
    let found = null;
    if (source) {
      try {
        found = await foodGet(source);
      } catch {
        found = null; // the pack is not here: fall back to the entry's own numbers
      }
    }
    portion = {
      title,
      grams: grams || found?.portions[0]?.grams || 100,
      per100: found?.per100 ?? null,
      base: numbersOf(props),
      baseGrams: grams,
      portions: found?.portions ?? [],
      source,
    };
  }
  async function openHit(h: FoodHit) {
    failed = '';
    try {
      const f = await foodGet(h.id);
      if (!f) throw new Error('that food is no longer in the list');
      portion = {
        title: f.name,
        grams: f.portions[0]?.grams ?? 100,
        per100: f.per100,
        base: {},
        baseGrams: 0,
        portions: f.portions,
        source: `${f.pack}/${f.id}`,
      };
    } catch (e) {
      failed = String(e);
    }
  }
  onMount(() => {
    if (entry) void openPortion(entry.title ?? '', entry.props);
  });
  /// While an ingredient is being chosen, a serving of another recipe is not offered as one.
  const offered = $derived((recent ?? []).filter((r) => !picking || whole(r.props[PCT]) === null));

  const stepSize = (g: number) => (g < 50 ? 5 : 10);
  function nudge(dir: 1 | -1) {
    if (!portion) return;
    portion.grams = Math.max(1, portion.grams + dir * stepSize(portion.grams));
  }
  /// Press and hold repeats, as a stepper should.
  let holding: ReturnType<typeof setInterval> | null = null;
  function hold(dir: 1 | -1) {
    nudge(dir);
    release();
    holding = setInterval(() => nudge(dir), 110);
  }
  function release() {
    if (holding) clearInterval(holding);
    holding = null;
  }
  $effect(() => release);

  // ── writing ────────────────────────────────────────────────────────────────────────────────────
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
  const draft = (title: string, props: Record<string, string | number>): NoteDraft => ({
    title,
    // **The time is taken, never asked for**: logged today, an entry is of now. For another day
    // nobody knows when it was, so it carries the day alone until the person says.
    start: day === today() ? `${day}T${nowClock()}` : day,
    // For a day still to come it is a plan, not something eaten: in no total until confirmed.
    props: { log: def.log, slot: chosen, ...props, ...(day > today() ? { planned: 1 } : {}) },
  });

  /// The row's own plus: the same food, the same amount, no second screen.
  const addAgain = (r: RecentItem) =>
    write(async () => {
      const { slot: _slot, log: _log, planned: _planned, ...rest } = r.props;
      const props = Object.fromEntries(
        Object.entries(rest).filter(
          (e): e is [string, string | number] =>
            typeof e[1] === 'string' || typeof e[1] === 'number',
        ),
      );
      await createNotesUndoable([draft(r.title, props)], `logged “${r.title}”`);
      added += 1;
      onwrote();
    });

  const addPortion = () =>
    write(async () => {
      if (!portion) return;
      const p = portion;
      if (into) {
        // An ingredient for a recipe that is already kept: one more entry under its word.
        const r = into;
        const props = { [AMOUNT]: p.grams, ...valuesAt(p, p.grams) };
        await createNotesUndoable(
          [
            {
              title: p.title,
              props: {
                log: def.log,
                recipe: r.recipe,
                ...props,
                ...(p.source ? { source: p.source } : {}),
              },
            },
          ],
          `added “${p.title}” to “${r.title}”`,
        );
        portion = null;
        picking = false;
        into = null;
        await reopen(r.recipe);
        return;
      }
      if (building) {
        // An ingredient, not something eaten: it goes in the recipe being put together.
        building.items.push({
          title: p.title,
          grams: p.grams,
          values: valuesAt(p, p.grams),
          source: p.source,
        });
        portion = null;
        picking = false;
        return;
      }
      const props: Record<string, string | number> = {
        [AMOUNT]: p.grams,
        ...valuesAt(p, p.grams),
        ...rangeProps(p.grams),
      };
      if (p.source) props.source = p.source;
      await createNotesUndoable([draft(p.title, props)], `logged “${p.title}”`);
      added += 1;
      portion = null;
      onwrote();
    });

  const save = () =>
    write(async () => {
      if (!portion || !entry) return;
      const p = portion;
      const was = clockOf(entry.start);
      await setPropertiesUndoable(
        entry.id,
        {
          [AMOUNT]: p.grams,
          slot: chosen,
          ...valuesAt(p, p.grams),
          ...rangeProps(p.grams),
          // Only when the time was changed: the day is the entry's own, never the screen's.
          ...(at !== was
            ? { start: at ? `${dayOf(entry.start) || day}T${at}` : dayOf(entry.start) || day }
            : {}),
        },
        `changed “${p.title}”`,
      );
      onwrote();
      onclose();
    });

  const remove = () =>
    write(async () => {
      if (!entry) return;
      const title = entry.title ?? 'an entry';
      await deleteUndoable(entry.id, title);
      onwrote({ removed: title });
      onclose();
    });

  // ── recipes ────────────────────────────────────────────────────────────────────────────────────
  // A recipe is saved, not eaten: a note with `servings`, and one note per ingredient, all sharing
  // one word under `recipe` and none of them on a day. What it comes to is the sum of what went
  // in, as entered — nothing here estimates what cooking does to a weight or to a vitamin
  // (`decisions.md#data`, 2026-10-11). A serving logged from it is an ordinary entry.
  const PCT = 'servings_pct';
  let recipes = $state<Recipe[] | null>(null);
  const loadRecipes = () =>
    logRecipes(view)
      .then((r) => (recipes = r))
      .catch((e) => (failed = String(e)));
  $effect(() => {
    if (!entry && mode === 'recipes' && untrack(() => recipes) === null) void loadRecipes();
  });
  /// A recipe's values, the amount left out: the weight that went in is not the weight served.
  const wholeOf = (r: Recipe) =>
    Object.fromEntries(
      Object.entries(r.sums)
        .filter(([k]) => k !== AMOUNT)
        .map(([k, v]) => [k, v.sum]),
    );
  /// What is missing from a total, in words: “Iron from 1 of 2 ingredients”.
  const gaps = (of: number, n: (key: string) => number) =>
    def.metrics
      .filter((m) => n(m.key) < of)
      .map((m) => `${m.label} from ${n(m.key)} of ${of} ingredients`);

  type Serving = {
    title: string;
    word: string;
    /** How much of one serving, in hundredths. */
    pct: number;
    /** The whole recipe and how many servings it makes, when it is still kept. */
    all: Record<string, number> | null;
    servings: number;
    /** Otherwise: the values as they were at `basePct`. */
    base: Record<string, number>;
    basePct: number;
    made: Recipe | null;
  };
  let serving = $state<Serving | null>(null);
  function servingAt(s: Serving, pct: number): Record<string, number> {
    const scale = (v: number, by: number, over: number) => Math.round((v * by) / over);
    if (s.all)
      return Object.fromEntries(
        Object.entries(s.all).map(([k, v]) => [k, scale(v, pct, 100 * s.servings)]),
      );
    if (s.basePct <= 0) return s.base;
    return Object.fromEntries(
      Object.entries(s.base).map(([k, v]) => [k, scale(v, pct, s.basePct)]),
    );
  }
  const served = $derived(serving ? servingAt(serving, serving.pct) : {});
  const openRecipe = (r: Recipe) =>
    (serving = {
      title: r.title,
      word: r.recipe,
      pct: 100,
      all: wholeOf(r),
      servings: r.servings,
      base: {},
      basePct: 0,
      made: r,
    });
  async function openServing(title: string, props: Record<string, unknown>) {
    failed = '';
    const word = typeof props.source === 'string' ? props.source.replace(/^recipe\//, '') : '';
    let made: Recipe | null = null;
    try {
      made = (await logRecipes(view)).find((r) => r.recipe === word) ?? null;
    } catch {
      made = null; // fall back to the entry's own numbers
    }
    const pct = whole(props[PCT]) ?? 100;
    const base = Object.fromEntries(
      Object.entries(props).flatMap(([k, v]) => {
        const n = whole(v);
        return n === null || k === PCT || k === 'planned' ? [] : [[k, n] as const];
      }),
    );
    serving = {
      title,
      word,
      pct,
      all: made ? wholeOf(made) : null,
      servings: made?.servings ?? 1,
      base,
      basePct: pct,
      made,
    };
  }
  const QUARTER = 25;
  const lessOrMore = (dir: 1 | -1) =>
    serving && (serving.pct = Math.max(QUARTER, serving.pct + dir * QUARTER));

  const addServing = () =>
    write(async () => {
      if (!serving) return;
      const s = serving;
      const props = { [PCT]: s.pct, source: `recipe/${s.word}`, ...servingAt(s, s.pct) };
      await createNotesUndoable([draft(s.title, props)], `logged “${s.title}”`);
      added += 1;
      serving = null;
      onwrote();
    });
  const saveServing = () =>
    write(async () => {
      if (!serving || !entry) return;
      const s = serving;
      await setPropertiesUndoable(
        entry.id,
        { [PCT]: s.pct, slot: chosen, ...servingAt(s, s.pct) },
        `changed “${s.title}”`,
      );
      onwrote();
      onclose();
    });
  const removeRecipe = () =>
    write(async () => {
      const r = serving?.made;
      if (!r) return;
      await deleteNotesUndoable(
        [r.id, ...r.ingredients.map((i) => i.id)],
        `removed the recipe “${r.title}”`,
      );
      serving = null;
      await loadRecipes();
    });

  // ── changing a recipe that is kept ─────────────────────────────────────────────────────────────
  /// The recipe an ingredient is being chosen for.
  let into = $state<Recipe | null>(null);
  /// The ingredient whose amount is being changed, and whose recipe it is.
  let fixing = $state<{ word: string; id: string; title: string } | null>(null);
  /// Read the recipes again and show this one as it now is.
  async function reopen(word: string) {
    await loadRecipes();
    const r = (recipes ?? []).find((x) => x.recipe === word);
    if (r) openRecipe(r);
    else serving = null;
  }
  function addIngredientTo(r: Recipe) {
    into = r;
    serving = null;
    picking = true;
    mode = 'search';
  }
  async function fixIngredient(r: Recipe, i: ObjectMeta) {
    fixing = { word: r.recipe, id: i.id, title: i.title ?? 'an ingredient' };
    serving = null;
    await openPortion(i.title ?? '', i.props);
  }
  const leaveFix = async () => {
    const f = fixing;
    fixing = null;
    portion = null;
    if (f) await reopen(f.word);
  };
  const saveIngredient = () =>
    write(async () => {
      if (!portion || !fixing) return;
      const p = portion;
      await setPropertiesUndoable(
        fixing.id,
        { [AMOUNT]: p.grams, ...valuesAt(p, p.grams) },
        `changed “${p.title}”`,
      );
      await leaveFix();
    });
  const removeIngredient = () =>
    write(async () => {
      if (!fixing) return;
      await deleteUndoable(fixing.id, fixing.title);
      await leaveFix();
    });
  /// How many servings a kept recipe makes, as typed.
  let makesNow = $state<number | string | null>(null);
  $effect(() => {
    makesNow = serving?.made?.servings ?? null;
  });
  const saveServings = () =>
    write(async () => {
      const r = serving?.made;
      const n = Math.floor(Number(makesNow));
      if (!r || !(n >= 1) || n === r.servings) return;
      await setPropertiesUndoable(r.id, { servings: n }, `changed what “${r.title}” makes`);
      await reopen(r.recipe);
    });

  // Putting a recipe together: a name, how many servings it makes, and foods by weight.
  type Item = { title: string; grams: number; values: Record<string, number>; source: string };
  let building = $state<{
    name: string;
    servings: number | string | null;
    items: Item[];
  } | null>(null);
  /// Choosing an ingredient: the same search and the same amount step as logging a food.
  let picking = $state(false);
  const makes = $derived(Math.floor(Number(building?.servings ?? 0)));
  const built = $derived.by(() => {
    const sum: Record<string, number> = {};
    for (const i of building?.items ?? [])
      for (const [k, v] of Object.entries(i.values)) sum[k] = (sum[k] ?? 0) + v;
    return sum;
  });
  const perServing = $derived(
    makes >= 1
      ? Object.fromEntries(Object.entries(built).map(([k, v]) => [k, Math.round(v / makes)]))
      : {},
  );
  const recipeReady = $derived(
    !!building && building.name.trim() !== '' && makes >= 1 && building.items.length > 0,
  );
  const saveRecipe = () =>
    write(async () => {
      if (!building || !recipeReady) return;
      const b = building;
      const recipe = recipeWord();
      const name = b.name.trim();
      const drafts: NoteDraft[] = [
        // The recipe's own note is an ordinary one: tagged, in the Library, with no `log`.
        { title: name, tags: [RECIPE], props: { recipe, servings: makes } },
        ...b.items.map((i) => ({
          title: i.title,
          props: {
            log: def.log,
            recipe,
            [AMOUNT]: i.grams,
            ...i.values,
            ...(i.source ? { source: i.source } : {}),
          },
        })),
      ];
      await createNotesUndoable(drafts, `saved the recipe “${name}”`);
      building = null;
      mode = 'recipes';
      await loadRecipes();
    });

  // ── typing a food in ───────────────────────────────────────────────────────────────────────────
  let name = $state('');
  // A number field hands back a number, or nothing at all when it is empty.
  let typedIn = $state<Record<string, number | string | null | undefined>>({});
  let more = $state(false);
  const quickFields = $derived<Metric[]>(more ? [primary, ...secondary] : [primary]);
  const quickReady = $derived(name.trim() !== '' && Number(typedIn[primary.key]) > 0);
  const addQuick = () =>
    write(async () => {
      const props: Record<string, number> = {};
      for (const m of quickFields) {
        const raw = typedIn[m.key];
        if (raw === '' || raw === null || raw === undefined) continue;
        const n = Number(raw);
        if (Number.isFinite(n) && n >= 0) props[m.key] = Math.round(n * typed(m.key).per);
      }
      await createNotesUndoable([draft(name.trim(), props)], `logged “${name.trim()}”`);
      added += 1;
      name = '';
      typedIn = {};
      onwrote();
    });

  // ── dragging the sheet away by its grabber ─────────────────────────────────────────────────────
  let pull = $state(0);
  let from: number | null = null;
  function grab(e: PointerEvent) {
    from = e.clientY;
    (e.currentTarget as HTMLElement).setPointerCapture?.(e.pointerId);
  }
  function drag(e: PointerEvent) {
    if (from !== null) pull = Math.max(0, e.clientY - from);
  }
  function drop() {
    if (from === null) return;
    from = null;
    if (pull > 90) onclose();
    else pull = 0;
  }

  const summary = (v: Record<string, number>) =>
    [primary, ...secondary]
      .filter((m) => m.key in v)
      .map((m) =>
        m === primary ? showWith(m.key, v[m.key]) : `${m.label} ${showWith(m.key, v[m.key])}`,
      );
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<div class="log-overlay">
  <button type="button" class="scrim" onclick={onclose} aria-label="close" tabindex="-1"></button>
  <div
    class="sheet"
    class:dragging={pull > 0}
    role="dialog"
    aria-modal="true"
    aria-label={entry ? `change ${entry.title ?? 'entry'}` : 'log a food'}
    style="transform: translateY({pull}px)"
  >
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="grab"
      onpointerdown={grab}
      onpointermove={drag}
      onpointerup={drop}
      onpointercancel={drop}
    >
      <span></span>
    </div>

    <header class="top">
      {#if def.slots.length}
        <label class="slot">
          <span class="sr">meal</span>
          <select bind:value={chosen}>
            {#if !slot && !entry}<option value="" disabled>Which meal?</option>{/if}
            {#each def.slots as s (s.value)}
              <option value={s.value}>{s.label}</option>
            {/each}
          </select>
        </label>
      {/if}
      {#if added > 0}<span class="added" role="status">{added} added</span>{/if}
      <button type="button" class="x" onclick={onclose} aria-label="close">
        <Icon name="close" size={18} />
      </button>
    </header>

    {#if failed}<p class="failed" role="alert">{failed}</p>{/if}

    {#if portion}
      <!-- One food, how much of it, and what that comes to. -->
      <section class="portion">
        <h2>{portion.title}</h2>
        <div class="stepper">
          <button
            type="button"
            class="round"
            aria-label="less"
            onpointerdown={() => hold(-1)}
            onpointerup={release}
            onpointerleave={release}
            onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && nudge(-1)}
          >
            <Icon name="minus" size={22} />
          </button>
          <div class="grams">
            <input
              type="number"
              inputmode="numeric"
              min="1"
              aria-label="amount in grams"
              bind:value={portion.grams}
            />
            <span class="unit">g</span>
          </div>
          <button
            type="button"
            class="round"
            aria-label="more"
            onpointerdown={() => hold(1)}
            onpointerup={release}
            onpointerleave={release}
            onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && nudge(1)}
          >
            <Icon name="plus" size={22} />
          </button>
        </div>
        {#if portion.portions.length}
          <div class="chips">
            {#each portion.portions as p (p.label)}
              <button
                type="button"
                class="chip"
                class:on={portion.grams === p.grams}
                onclick={() => portion && (portion.grams = p.grams)}>{p.label}</button
              >
            {/each}
          </div>
        {/if}
        <p class="comes-to">
          {#each summary(now) as part, i (i)}
            <span class:lead={i === 0}>{part}</span>
          {/each}
        </p>
        {#if !fixing && !building && !into}
          {#if unsure}
            <div class="range">
              <label>
                <span>At least (g)</span>
                <input class="field" type="number" inputmode="numeric" min="1" bind:value={least} />
              </label>
              <label>
                <span>At most (g)</span>
                <input class="field" type="number" inputmode="numeric" min="1" bind:value={most} />
              </label>
            </div>
            <p class="hint">
              {#if Number(least) > 0 && Number(most) > Number(least) && (Number(least) > portion.grams || Number(most) < portion.grams)}
                The amount above has to lie between the two.
              {:else}
                The day's totals will say the least and the most they could be.
              {/if}
              <button type="button" class="link" onclick={() => (unsure = false)}>
                One amount
              </button>
            </p>
          {:else}
            <button type="button" class="link" onclick={() => (unsure = true)}>
              Not sure of the amount?
            </button>
          {/if}
        {/if}
        {#if fixing}
          <div class="pair">
            <button type="button" class="quiet" onclick={removeIngredient} disabled={busy}>
              Take out
            </button>
            <button type="button" class="go" onclick={saveIngredient} disabled={busy}>Save</button>
          </div>
          <button type="button" class="link" onclick={leaveFix}>Back to the recipe</button>
        {:else if entry}
          <label class="when">
            <span>Time</span>
            <input class="field" type="time" bind:value={at} />
          </label>
          <div class="pair">
            <button type="button" class="quiet" onclick={remove} disabled={busy}>Remove</button>
            <button type="button" class="go" onclick={save} disabled={busy}>Save</button>
          </div>
        {:else}
          <div class="pair">
            <button type="button" class="quiet" onclick={() => (portion = null)}>Back</button>
            {#if building || into}
              <button type="button" class="go" onclick={addPortion} disabled={busy}>
                Add to the recipe
              </button>
            {:else}
              <button type="button" class="go" onclick={addPortion} disabled={busy || unplaced}>
                Add{slotLabel ? ` to ${slotLabel}` : ''}
              </button>
            {/if}
          </div>
        {/if}
      </section>
    {:else if serving}
      <!-- One recipe, how much of a serving, and what that comes to. -->
      <section class="portion">
        <h2>{serving.title}</h2>
        <div class="stepper">
          <button type="button" class="round" aria-label="less" onclick={() => lessOrMore(-1)}>
            <Icon name="minus" size={22} />
          </button>
          <div class="grams">
            <output aria-label="servings">{serving.pct / 100}</output>
            <span class="unit">{serving.pct === 100 ? 'serving' : 'servings'}</span>
          </div>
          <button type="button" class="round" aria-label="more" onclick={() => lessOrMore(1)}>
            <Icon name="plus" size={22} />
          </button>
        </div>
        <p class="comes-to">
          {#each summary(served) as part, i (i)}
            <span class:lead={i === 0}>{part}</span>
          {/each}
        </p>
        {#if serving.made}
          {@const r = serving.made}
          <p class="hint">
            The sum of its ingredients as entered — nothing is adjusted for cooking.
          </p>
          {#each gaps(r.of, (k) => r.sums[k]?.n ?? 0) as g (g)}<p class="hint">{g}</p>{/each}
          {#if entry}
            <ul class="rows">
              {#each r.ingredients as i (i.id)}
                <li class="plain">
                  <span class="t">{i.title}</span>
                  <span class="a">{showWith(AMOUNT, whole(i.props[AMOUNT]) ?? 0)}</span>
                </li>
              {/each}
            </ul>
          {:else}
            <!-- The recipe itself, changed here: what it makes, and what goes in it. -->
            <label class="makes">
              <span>Servings it makes</span>
              <input
                class="field"
                type="number"
                inputmode="numeric"
                min="1"
                step="1"
                bind:value={makesNow}
                onchange={saveServings}
              />
            </label>
            {#if r.of === 0}<p class="hint">Nothing in it yet.</p>{/if}
            <ul class="rows">
              {#each r.ingredients as i (i.id)}
                <li>
                  <button
                    type="button"
                    class="row"
                    onclick={() => fixIngredient(r, i)}
                    aria-label={`change ${i.title ?? 'ingredient'}`}
                  >
                    <span class="t">{i.title}</span>
                    <span class="a">{showWith(AMOUNT, whole(i.props[AMOUNT]) ?? 0)}</span>
                    <span class="v">{show(primary.key, whole(i.props[primary.key]) ?? 0).text}</span
                    >
                  </button>
                </li>
              {/each}
            </ul>
            <button type="button" class="link" onclick={() => addIngredientTo(r)}>
              Add an ingredient
            </button>
          {/if}
        {/if}
        {#if entry}
          <div class="pair">
            <button type="button" class="quiet" onclick={remove} disabled={busy}>Remove</button>
            <button type="button" class="go" onclick={saveServing} disabled={busy}>Save</button>
          </div>
        {:else}
          <div class="pair">
            <button type="button" class="quiet" onclick={() => (serving = null)}>Back</button>
            <button type="button" class="go" onclick={addServing} disabled={busy || unplaced}>
              Add{slotLabel ? ` to ${slotLabel}` : ''}
            </button>
          </div>
          {#if serving.made}
            <button type="button" class="link" onclick={removeRecipe} disabled={busy}>
              Remove this recipe
            </button>
          {/if}
        {/if}
      </section>
    {:else if building && !picking}
      <section class="quick">
        <h2>New recipe</h2>
        <label>
          <span>Name</span>
          <input class="field" type="text" bind:value={building.name} autocomplete="off" />
        </label>
        <label>
          <span>Servings it makes</span>
          <input
            class="field"
            type="number"
            inputmode="numeric"
            min="1"
            step="1"
            bind:value={building.servings}
          />
        </label>
        <ul class="rows">
          {#each building.items as i, at (at)}
            <li>
              <span class="row still">
                <span class="t">{i.title}</span>
                <span class="a">{showWith(AMOUNT, i.grams)}</span>
                <span class="v">{show(primary.key, i.values[primary.key] ?? 0).text}</span>
              </span>
              <button
                type="button"
                class="again"
                onclick={() => building?.items.splice(at, 1)}
                aria-label={`take ${i.title} out`}
              >
                <Icon name="close" size={18} />
              </button>
            </li>
          {/each}
        </ul>
        <button type="button" class="link" onclick={() => ((picking = true), (mode = 'search'))}>
          Add an ingredient
        </button>
        {#if building.items.length && makes >= 1}
          <p class="comes-to">
            {#each summary(perServing) as part, i (i)}
              <span class:lead={i === 0}>{part}</span>
            {/each}
          </p>
          <p class="hint">
            Per serving: the sum of the ingredients as entered, divided by {makes}.
          </p>
          {#each gaps(building.items.length, (k) => building?.items.filter((i) => k in i.values).length ?? 0) as g (g)}
            <p class="hint">{g}</p>
          {/each}
        {/if}
        <div class="pair">
          <button type="button" class="quiet" onclick={() => (building = null)}>Cancel</button>
          <button type="button" class="go" onclick={saveRecipe} disabled={busy || !recipeReady}>
            Save recipe
          </button>
        </div>
      </section>
    {:else if reading}
      <section class="quick" aria-label="a recipe, as published">
        <h2>{reading.title}</h2>
        {#if reading.yield}<p class="hint">{reading.yield}</p>{/if}
        <p class="comes-to">
          {#each summary(reading.per_serving) as part, i (i)}
            <span class:lead={i === 0}>{part}</span>
          {/each}
        </p>
        <p class="hint">A serving, by the Institute's own figures. Nothing here is worked out.</p>
        <h3>Ingredients</h3>
        <ul class="asis">
          {#each reading.ingredients as line, i (i)}<li>{line}</li>{/each}
        </ul>
        <h3>Directions</h3>
        <ol class="asis">
          {#each reading.steps as step, i (i)}<li>{step}</li>{/each}
        </ol>
        <p class="hint">
          Source: {carried?.source ?? ''}. Ingredients and directions as published; the
          <a href={reading.url} target="_blank" rel="noreferrer noopener">recipe's own page</a> has its
          tips and notes.
        </p>
        <div class="pair">
          <button type="button" class="quiet" onclick={() => (reading = null)}>Back</button>
          <button type="button" class="go" onclick={addCarried} disabled={busy || unplaced}>
            Add a serving{slotLabel ? ` to ${slotLabel}` : ''}
          </button>
        </div>
      </section>
    {:else if labelling}
      <section class="quick" aria-label="a packaged food, from its label">
        <h2>From the label</h2>
        <label>
          <span>Name</span>
          <input class="field" type="text" bind:value={labelling.name} autocomplete="off" />
        </label>
        <label>
          <span>Barcode number (optional)</span>
          <input
            class="field"
            type="text"
            inputmode="numeric"
            bind:value={labelling.barcode}
            autocomplete="off"
          />
        </label>
        {#if badCode}
          <p class="hint">
            That is not a barcode number: its last digit does not check. Leave it empty if unsure.
          </p>
        {:else if labelling.barcode.trim()}
          <button type="button" class="quiet" onclick={lookUp} disabled={looking}>
            {looking ? 'Asking…' : 'Look it up at Open Food Facts'}
          </button>
          <p class="hint">
            This sends the barcode number, and nothing else, to
            <a href="https://world.openfoodfacts.org" target="_blank" rel="noreferrer noopener"
              >Open Food Facts</a
            >, a public database kept by volunteers (<a
              href="https://opendatacommons.org/licenses/odbl/1-0/"
              target="_blank"
              rel="noreferrer noopener">Open Database License</a
            >). Its data can contain errors and is not for medical use.
          </p>
        {/if}
        {#if lookedUp}<p class="hint" role="status">{lookedUp}</p>{/if}
        <p class="hint">
          Per 100 g, as the label prints it. Energy is needed; the rest as you like.
        </p>
        {#each LABEL as f (f.key)}
          <label>
            <span>{f.label}</span>
            <input
              class="field"
              type="number"
              inputmode="decimal"
              min="0"
              step="0.1"
              bind:value={labelling.values[f.key]}
            />
          </label>
        {/each}
        <label>
          <span>A serving, in grams (optional)</span>
          <input
            class="field"
            type="number"
            inputmode="numeric"
            min="1"
            bind:value={labelling.serving}
          />
        </label>
        <label>
          <span>Ingredients, as on the pack (optional)</span>
          <textarea class="field tall" rows="3" bind:value={labelling.ingredients}></textarea>
        </label>
        <div class="pair">
          <button type="button" class="quiet" onclick={() => (labelling = null)}>Back</button>
          <button type="button" class="go" onclick={saveLabel} disabled={busy || !labelReady}>
            Save it
          </button>
        </div>
      </section>
    {:else}
      {#if picking}
        <button
          type="button"
          class="link"
          onclick={() => {
            picking = false;
            const r = into;
            into = null;
            if (r) openRecipe(r);
          }}
        >
          Back to the recipe
        </button>
      {/if}
      <div class="modes" role="group" aria-label="how to add">
        <button type="button" class:on={mode === 'recent'} onclick={() => (mode = 'recent')}
          >Recent</button
        >
        <button type="button" class:on={mode === 'search'} onclick={() => (mode = 'search')}
          >Search</button
        >
        {#if !picking}
          <button type="button" class:on={mode === 'recipes'} onclick={() => (mode = 'recipes')}
            >Recipes</button
          >
          <button type="button" class:on={mode === 'quick'} onclick={() => (mode = 'quick')}
            >Quick add</button
          >
        {/if}
      </div>

      {#if mode === 'recent'}
        {#if recent === null}
          <ul class="rows" aria-hidden="true">
            {#each [0, 1, 2, 3] as i (i)}<li class="ghost"></li>{/each}
          </ul>
        {:else if offered.length === 0}
          <p class="hint">Nothing logged yet. Search for a food, or type one in.</p>
        {:else}
          <ul class="rows">
            {#each offered as r (r.title + String(r.props.source ?? ''))}
              {@const grams = whole(r.props[AMOUNT])}
              <li>
                <button type="button" class="row" onclick={() => openPortion(r.title, r.props)}>
                  <span class="t">{r.title}</span>
                  <span class="a">{grams !== null ? showWith(AMOUNT, grams) : ''}</span>
                  <span class="v">{show(primary.key, whole(r.props[primary.key]) ?? 0).text}</span>
                </button>
                {#if !picking}
                  <button
                    type="button"
                    class="again"
                    onclick={() => addAgain(r)}
                    disabled={busy || unplaced}
                    aria-label={`add ${r.title} again`}
                  >
                    <Icon name="plus" size={18} />
                  </button>
                {/if}
              </li>
            {/each}
          </ul>
        {/if}
      {:else if mode === 'search'}
        <!-- svelte-ignore a11y_autofocus -->
        <input
          class="field"
          type="search"
          placeholder="Search foods"
          aria-label="search foods"
          bind:value={q}
          autofocus
          spellcheck="false"
        />
        {#if q.trim() && hits.length === 0 && mine.length === 0}
          <p class="hint">No food by that name. Quick add takes any name.</p>
        {/if}
        <ul class="rows">
          {#each mine as p (p.id)}
            <li>
              <button type="button" class="row" onclick={() => openProduct(p)}>
                <span class="t">{p.title}</span>
                <span class="a">Your label · per 100 g</span>
                <span class="v">{show(primary.key, p.per100[primary.key] ?? 0).text}</span>
              </button>
            </li>
          {/each}
          {#each hits as h (h.id)}
            <li>
              <button type="button" class="row" onclick={() => openHit(h)}>
                <span class="t">{h.name}</span>
                <!-- Which table: two may both list a banana, and they are not one measurement. -->
                <span class="a">{TABLES[h.src] ?? h.src} · per 100 g</span>
                <span class="v">{show(primary.key, h.kcal).text}</span>
              </button>
            </li>
          {/each}
        </ul>
        <button type="button" class="link" onclick={startLabel}>
          {isBarcode(q) && mine.length === 0
            ? 'A barcode you have not written down: add it'
            : 'A packaged food? Write it down from its label'}
        </button>
      {:else if mode === 'recipes'}
        {#if recipes === null}
          <ul class="rows" aria-hidden="true">
            {#each [0, 1] as i (i)}<li class="ghost"></li>{/each}
          </ul>
        {:else}
          {#if recipes.length === 0}
            <p class="hint">
              No recipes yet. A recipe is foods by weight and the servings it makes; what a serving
              comes to is added up for you.
            </p>
          {/if}
          <ul class="rows">
            {#each recipes as r (r.id)}
              <li>
                <button type="button" class="row" onclick={() => openRecipe(r)}>
                  <span class="t">{r.title}</span>
                  <span class="a">1 of {r.servings} servings</span>
                  <span class="v"
                    >{show(primary.key, Math.round((r.sums[primary.key]?.sum ?? 0) / r.servings))
                      .text}</span
                  >
                </button>
              </li>
            {/each}
          </ul>
          <button
            type="button"
            class="go wide"
            onclick={() => (building = { name: '', servings: 1, items: [] })}
          >
            New recipe
          </button>
          {#if carried?.recipes.length}
            <h3 class="shelf">From the NHLBI ({carried.recipes.length})</h3>
            <ul class="rows">
              {#each carried.recipes as r (r.id)}
                <li>
                  <button type="button" class="row" onclick={() => (reading = r)}>
                    <span class="t">{r.title}</span>
                    <span class="a">1 serving{r.servings ? ` of ${r.servings}` : ''}</span>
                    <span class="v">{show(primary.key, r.per_serving[primary.key] ?? 0).text}</span>
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        {/if}
      {:else}
        <div class="quick">
          <label>
            <span>Name</span>
            <input class="field" type="text" bind:value={name} autocomplete="off" />
          </label>
          {#each quickFields as m (m.key)}
            <label>
              <span>{m.label}{typed(m.key).unit ? ` (${typed(m.key).unit})` : ''}</span>
              <input
                class="field"
                type="number"
                inputmode="decimal"
                min="0"
                bind:value={typedIn[m.key]}
              />
            </label>
          {/each}
          {#if !more && secondary.length}
            <button type="button" class="link" onclick={() => (more = true)}>
              Add {secondary.map((m) => m.label.toLowerCase()).join(', ')}
            </button>
          {/if}
          <button
            type="button"
            class="go wide"
            onclick={addQuick}
            disabled={busy || !quickReady || unplaced}
          >
            Add{slotLabel ? ` to ${slotLabel}` : ''}
          </button>
        </div>
      {/if}
    {/if}
  </div>
</div>

<style>
  .log-overlay {
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
    animation: scrim-in var(--dur-med) var(--ease);
  }
  .sheet {
    position: relative;
    width: min(34rem, 100%);
    max-height: 100%;
    overflow-y: auto;
    overscroll-behavior: contain;
    box-sizing: border-box;
    padding: 0 var(--space-4) max(var(--safe-bottom), var(--space-4));
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    background: var(--surface-elevated);
    border-radius: var(--radius-lg) var(--radius-lg) 0 0;
    box-shadow: var(--shadow-lg);
    animation: sheet-up var(--dur-med) var(--ease);
    transition: transform var(--dur-med) var(--ease);
  }
  .sheet.dragging {
    transition: none;
  }
  @keyframes sheet-up {
    from {
      transform: translateY(100%);
    }
  }
  @keyframes scrim-in {
    from {
      opacity: 0;
    }
  }
  button,
  input,
  select {
    font: inherit;
    color: inherit;
  }
  .grab {
    position: sticky;
    top: 0;
    z-index: 1;
    display: grid;
    place-items: center;
    height: 1.75rem;
    margin: 0 calc(var(--space-4) * -1);
    background: var(--surface-elevated);
    cursor: grab;
    touch-action: none;
  }
  .grab span {
    width: 2.5rem;
    height: 4px;
    border-radius: var(--radius-pill);
    background: var(--border-strong);
  }
  .top {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .slot {
    flex: 1;
  }
  .slot select {
    min-height: 2.5rem;
    padding: 0 var(--space-2);
    border: 0;
    border-radius: var(--radius-md);
    background: none;
    font-size: var(--text-lg);
    font-weight: 600;
  }
  .added {
    font-size: var(--text-sm);
    color: var(--ok-fg);
    background: var(--ok-bg);
    padding: 2px var(--space-2);
    border-radius: var(--radius-pill);
  }
  .x {
    margin-left: auto;
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
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
  .failed {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--danger-fg);
  }
  .hint {
    margin: 0;
    padding: var(--space-4) 0;
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

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
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow-sm);
  }

  .rows {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .rows li {
    display: flex;
    align-items: center;
    border-bottom: 1px solid var(--border);
  }
  .ghost {
    height: 3rem;
    margin: var(--space-1) 0;
    border-radius: var(--radius-md);
    background: var(--surface-hover);
    border-bottom: 0 !important;
  }
  .row {
    flex: 1;
    min-width: 0;
    min-height: 3rem;
    display: grid;
    grid-template-columns: 1fr auto 3rem;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) 0;
    border: 0;
    background: none;
    text-align: left;
    cursor: pointer;
  }
  .t {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .a {
    color: var(--text-subtle);
    font-size: var(--text-sm);
  }
  .v {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .again {
    flex: none;
    margin-left: var(--space-3);
    width: 2.75rem;
    height: 2.75rem;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: var(--radius-pill);
    background: var(--accent-subtle);
    color: var(--accent);
    cursor: pointer;
  }
  .again:active {
    transform: scale(0.92);
  }

  .field {
    width: 100%;
    box-sizing: border-box;
    min-height: 2.75rem;
    padding: 0 var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
  }
  .quick {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .quick label {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .asis {
    margin: 0;
    padding-left: var(--space-5);
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  .shelf {
    margin: var(--space-3) 0 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .tall {
    min-height: 5rem;
    padding: var(--space-2) var(--space-3);
    font: inherit;
    resize: vertical;
  }
  .link {
    align-self: flex-start;
    padding: var(--space-1) 0;
    border: 0;
    background: none;
    color: var(--accent);
    cursor: pointer;
  }

  .portion {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }
  h2 {
    margin: 0;
    font-size: var(--text-lg);
  }
  .stepper {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-4);
  }
  .round {
    width: 3.25rem;
    height: 3.25rem;
    display: grid;
    place-items: center;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-pill);
    background: var(--surface);
    cursor: pointer;
    touch-action: manipulation;
  }
  .round:active {
    background: var(--surface-hover);
  }
  .grams {
    display: flex;
    align-items: baseline;
    gap: var(--space-1);
  }
  .grams input {
    width: 5.5rem;
    border: 0;
    background: none;
    text-align: right;
    font-size: 2.5rem;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    -moz-appearance: textfield;
    appearance: textfield;
  }
  .grams output {
    font-size: 2.5rem;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  /* A row that only tells: an ingredient in a recipe. */
  .rows li.plain {
    justify-content: space-between;
    gap: var(--space-3);
    min-height: 2.25rem;
    color: var(--text-muted);
  }
  .row.still {
    cursor: default;
  }
  .grams input::-webkit-outer-spin-button,
  .grams input::-webkit-inner-spin-button {
    -webkit-appearance: none;
  }
  .unit {
    color: var(--text-muted);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
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
  .comes-to {
    margin: 0;
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: var(--space-1) var(--space-3);
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }
  .comes-to .lead {
    flex-basis: 100%;
    text-align: center;
    font-size: var(--text-xl);
    font-weight: 600;
    color: var(--text);
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
  .go.wide {
    flex: none;
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
  .makes {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
  }
  .makes .field {
    width: 5rem;
    text-align: right;
  }
  .when {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .when .field {
    width: 9rem;
  }
  .range {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-2);
  }
  .range label {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .range .field {
    width: 100%;
    box-sizing: border-box;
  }
</style>
