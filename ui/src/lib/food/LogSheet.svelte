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
  import { logRecent, foodSearch, foodGet } from '../ipc';
  import { createNotesUndoable, setPropertiesUndoable, deleteUndoable } from '../undo.svelte';
  import { show, showWith, today, typed, whole } from '../units';
  import type {
    FoodHit,
    FoodPortion,
    Metric,
    NoteDraft,
    ObjectMeta,
    RecentItem,
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
  const primary = $derived(def.metrics.find((m) => m.role === 'primary') ?? def.metrics[0]);
  const secondary = $derived(def.metrics.filter((m) => m.role === 'secondary'));

  /// With no slot given, the part of the day it probably is: the day cut into as many pieces as
  /// there are slots between six in the morning and ten at night, anything later in the last.
  function byClock(): string {
    if (!def.slots.length) return '';
    const h = new Date().getHours();
    const main = Math.max(def.slots.length - 1, 1);
    const at =
      h < 6 || h >= 22
        ? def.slots.length - 1
        : Math.min(Math.floor(((h - 6) / 16) * main), main - 1);
    return def.slots[at].value;
  }
  // Read once, on purpose: the sheet is opened for one slot or one entry and closed again.
  let chosen = $state(
    untrack(() => (entry ? String(entry.props.slot ?? '') : (slot ?? byClock()))),
  );
  const slotLabel = $derived(def.slots.find((s) => s.value === chosen)?.label ?? '');

  type Mode = 'recent' | 'search' | 'quick';
  let mode = $state<Mode>('recent');
  let failed = $state('');
  let busy = $state(false);
  let added = $state(0);

  // ── what was eaten before ──────────────────────────────────────────────────────────────────────
  let recent = $state<RecentItem[] | null>(null);
  $effect(() => {
    if (entry) return;
    const s = chosen;
    logRecent(view, { slot: s, limit: 20 })
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
    start: day,
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
      const props: Record<string, string | number> = { [AMOUNT]: p.grams, ...valuesAt(p, p.grams) };
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
      await setPropertiesUndoable(
        entry.id,
        { [AMOUNT]: p.grams, slot: chosen, ...valuesAt(p, p.grams) },
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
        {#if entry}
          <div class="pair">
            <button type="button" class="quiet" onclick={remove} disabled={busy}>Remove</button>
            <button type="button" class="go" onclick={save} disabled={busy}>Save</button>
          </div>
        {:else}
          <div class="pair">
            <button type="button" class="quiet" onclick={() => (portion = null)}>Back</button>
            <button type="button" class="go" onclick={addPortion} disabled={busy}>
              Add{slotLabel ? ` to ${slotLabel}` : ''}
            </button>
          </div>
        {/if}
      </section>
    {:else}
      <div class="modes" role="group" aria-label="how to add">
        <button type="button" class:on={mode === 'recent'} onclick={() => (mode = 'recent')}
          >Recent</button
        >
        <button type="button" class:on={mode === 'search'} onclick={() => (mode = 'search')}
          >Search</button
        >
        <button type="button" class:on={mode === 'quick'} onclick={() => (mode = 'quick')}
          >Quick add</button
        >
      </div>

      {#if mode === 'recent'}
        {#if recent === null}
          <ul class="rows" aria-hidden="true">
            {#each [0, 1, 2, 3] as i (i)}<li class="ghost"></li>{/each}
          </ul>
        {:else if recent.length === 0}
          <p class="hint">Nothing logged yet. Search for a food, or type one in.</p>
        {:else}
          <ul class="rows">
            {#each recent as r (r.title + String(r.props.source ?? ''))}
              {@const grams = whole(r.props[AMOUNT])}
              <li>
                <button type="button" class="row" onclick={() => openPortion(r.title, r.props)}>
                  <span class="t">{r.title}</span>
                  <span class="a">{grams !== null ? showWith(AMOUNT, grams) : ''}</span>
                  <span class="v">{show(primary.key, whole(r.props[primary.key]) ?? 0).text}</span>
                </button>
                <button
                  type="button"
                  class="again"
                  onclick={() => addAgain(r)}
                  disabled={busy}
                  aria-label={`add ${r.title} again`}
                >
                  <Icon name="plus" size={18} />
                </button>
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
        {#if q.trim() && hits.length === 0}
          <p class="hint">No food by that name. Quick add takes any name.</p>
        {/if}
        <ul class="rows">
          {#each hits as h (h.id)}
            <li>
              <button type="button" class="row" onclick={() => openHit(h)}>
                <span class="t">{h.name}</span>
                <span class="a">per 100 g</span>
                <span class="v">{show(primary.key, h.kcal).text}</span>
              </button>
            </li>
          {/each}
        </ul>
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
          <button type="button" class="go wide" onclick={addQuick} disabled={busy || !quickReady}>
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
</style>
