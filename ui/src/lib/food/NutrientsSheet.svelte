<script lang="ts">
  // Every vitamin and mineral of a day, each beside the figure a named body gives for it
  // (`nutrients.rs`; `decisions.md#data`, 2026-10-12). The day screen names four; this is the rest.
  // Then the day's drinks: water, caffeine and alcohol (2026-10-31).
  //
  // A row says three things and no more: what the day came to, the reference where there is one
  // figure for this person, and — pressed — which entries it came from and, for each entry without
  // it, why (2026-10-29: "No entry of this day carries it" under choline read as the app failing).
  // An entry of the food list is read with the list's values as they are now (the server's
  // `packs::resolve`). A day still to come counts what is planned for it: it is its plan.
  import { onMount } from 'svelte';
  import Icon from '../Icon.svelte';
  import Meter from '../charts/Meter.svelte';
  import { logNutrients } from '../ipc';
  import { createNotesUndoable, setPropertiesUndoable } from '../undo.svelte';
  import { dayLabel, showWith, today } from '../units';
  import { borrowedOf, estimatedOf, valueOf } from './provenance';
  import { whyNot } from './refresh';
  import type { NutrientRow, Nutrients, ObjectMeta } from '../types';

  interface Props {
    view: string;
    day: string;
    /** Opened to choose what Day follows: every value as one line to tick, up to Day's most. */
    choose?: boolean;
    onclose: () => void;
  }
  let { view, day, choose = false, onclose }: Props = $props();

  let got = $state<Nutrients | null>(null);
  let failed = $state('');
  let opened = $state<string | null>(null);
  /// A day still to come is its plan, so what is planned for it counts.
  const load = async () => (got = await logNutrients(view, day, day > today()));
  onMount(async () => {
    try {
      await load();
    } catch (e) {
      failed = String(e);
    }
  });
  const KIND: Record<string, string> = {
    PRI: 'the intake that covers nearly everybody',
    AI: 'an adequate intake, set where a requirement could not be',
  };
  const from = (r: NutrientRow) =>
    (got?.entries ?? [])
      .filter((e) => (valueOf(e.props, r.key) ?? 0) > 0)
      .sort((a, b) => (valueOf(b.props, r.key) ?? 0) - (valueOf(a.props, r.key) ?? 0))
      .slice(0, 4);
  /// The entries without it, each with why.
  const without = (r: NutrientRow) =>
    (got?.entries ?? [])
      .filter((e: ObjectMeta) => valueOf(e.props, r.key) === null)
      .map((e) => ({ e, why: whyNot(e.props, r.key) }));
  const share = (r: NutrientRow) => (r.reference ? Math.round((r.sum / r.reference) * 100) : null);

  // **Show on Day** (`food-values-plan-2026-10-30.md`, step 4; `nutrients.rs` `pin`): a value of
  // this sheet joins Day's *Also today*, and Trends offers it. Kept in the profile note beside the
  // sex and the year of birth, so every device of the person's shows the same; at most six.
  let pinning = $state(false);
  async function toggle(key: string) {
    if (!got || pinning) return;
    const on = got.on_day.includes(key);
    const next = on ? got.on_day.filter((k) => k !== key) : [...got.on_day, key];
    // Every value taken off is written as `none`: an empty one would read as never chosen.
    const value = next.length ? next.join(', ') : 'none';
    const label = on ? 'took a value off Day' : 'put a value on Day';
    pinning = true;
    try {
      if (got.profile) await setPropertiesUndoable(got.profile, { day_values: value }, label);
      else
        await createNotesUndoable(
          [
            {
              title: 'About me',
              body: 'Your own measurements, used only to work out numbers you ask for.\n',
              props: { profile: 'me', day_values: value },
            },
          ],
          label,
        );
      await load();
    } catch (e) {
      failed = String(e);
    } finally {
      pinning = false;
    }
  }
</script>

{#snippet row(r: NutrientRow, of: number)}
  {@const pct = share(r)}
  <li>
    <button
      type="button"
      class="row"
      aria-expanded={opened === r.key}
      onclick={() => (opened = opened === r.key ? null : r.key)}
    >
      <span class="name">{r.label}</span>
      <span class="val"
        >{showWith(r.key, r.sum, r.reference ?? r.upper ?? r.sum)}{#if r.reference}<span class="sub"
            >&nbsp;/ {showWith(r.key, r.reference)}</span
          >{/if}</span
      >
      {#if r.reference}
        <Meter
          value={r.sum}
          target={r.reference}
          tint="a"
          label={`${r.label}: ${pct}% of the reference`}
        />
      {/if}
      {#if r.upper && r.sum > r.upper}
        <!-- An upper level is EFSA's and goes with a reference; a limit alone is WHO's for
             sodium, or EFSA's safe intake of caffeine. -->
        <span class="over"
          >above {r.kind ? 'the upper level' : 'the limit'} of {showWith(r.key, r.upper)}</span
        >
      {:else if r.upper && !r.reference}
        <span class="sub">limit {showWith(r.key, r.upper)}</span>
      {/if}
      {#if r.n < of}
        <span class="partial">from {r.n} of {of} entries</span>
      {/if}
    </button>
    {#if opened === r.key}
      <div class="why">
        {#each from(r) as e (e.id)}
          <p class="made">
            <span
              >{e.title ?? 'Untitled'}{#if estimatedOf(e.props)[r.key]}<span class="sub">
                  · estimated from {estimatedOf(e.props)[r.key]}</span
                >{:else if borrowedOf(e.props)[r.key]}<span class="sub">
                  · borrowed from {borrowedOf(e.props)[r.key]}</span
                >{/if}</span
            >
            <span
              >{estimatedOf(e.props)[r.key] ? '≈ ' : ''}{showWith(
                r.key,
                valueOf(e.props, r.key) ?? 0,
              )}</span
            >
          </p>
        {/each}
        {#each without(r) as { e, why } (e.id)}
          <p class="made missing">
            <span
              >{e.title ?? 'Untitled'}{#if why}<span class="sub"> · {why}</span>{/if}</span
            >
            <span>—</span>
          </p>
        {/each}
        {#if r.kind && KIND[r.kind]}<p class="sub">Reference: {KIND[r.kind]}.</p>{/if}
        {#if r.note}<p class="sub">{r.note}.</p>{/if}
        {#if got}
          {@const onDay = got.on_day.includes(r.key)}
          {@const full = !onDay && got.on_day.length >= got.most_on_day}
          <button
            type="button"
            class="pin"
            aria-pressed={onDay}
            disabled={pinning || full}
            onclick={() => toggle(r.key)}
            >{onDay ? 'Shown on Day · take it off' : 'Show on Day'}</button
          >
          {#if full}
            <p class="sub">Day shows {got.most_on_day} at most: take one off first.</p>
          {/if}
        {/if}
      </div>
    {/if}
  </li>
{/snippet}

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<div class="nutrients-overlay">
  <button type="button" class="scrim" onclick={onclose} aria-label="close" tabindex="-1"></button>
  <div class="sheet" role="dialog" aria-modal="true" aria-label="all nutrients">
    <header>
      <h2>{choose ? 'Follow on Day' : `Nutrients · ${dayLabel(day)}`}</h2>
      <button type="button" class="x" onclick={onclose} aria-label="close">
        <Icon name="close" size={18} />
      </button>
    </header>
    {#if failed}<p class="failed" role="alert">{failed}</p>{/if}
    {#if got && choose}
      <!-- **Choosing what Day follows** (2026-10-31, later): every value of the sheet, one line
           each, ticked when Day shows it; the order ticked is the order on Day. -->
      <p class="says">
        Up to {got.most_on_day}, shown as rings on the day, against your target or the European
        reference for you.
      </p>
      {#each [{ head: 'Drinks and water', list: got.drinks }, { head: 'Vitamins, minerals and fibre', list: got.rows }] as part (part.head)}
        <h3>{part.head}</h3>
        <ul>
          {#each part.list as r (r.key)}
            {@const on = got.on_day.includes(r.key)}
            <li>
              <button
                type="button"
                class="pick"
                aria-pressed={on}
                disabled={pinning || (!on && got.on_day.length >= got.most_on_day)}
                onclick={() => toggle(r.key)}
              >
                <span class="name">{r.label}</span>
                <span class="tick" aria-hidden="true">{on ? '✓' : ''}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/each}
      {#if got.on_day.length >= got.most_on_day}
        <p class="says">Day shows {got.most_on_day}: untick one to choose another.</p>
      {/if}
    {:else if got}
      {#if got.of === 0}
        <p class="says">Nothing is logged for this day.</p>
      {:else}
        {#if got.needs_sex}
          <p class="says">
            Some references differ for men and women and are left out until you say which, under
            Diet, in About you.
          </p>
        {/if}
        <ul>
          {#each got.rows as r (r.key)}{@render row(r, got.of)}{/each}
        </ul>
        <!-- **Drinks and water** (`nutrients.rs` `DRINKS`, 2026-10-31): the water of everything
             logged, as EFSA's total water counts it, the caffeine and the alcohol — the figures
             Health's Water and Caffeine cards use, and no target for alcohol. -->
        <h3>Drinks and water</h3>
        <ul>
          {#each got.drinks as r (r.key)}{@render row(r, got.of)}{/each}
        </ul>
        <p class="says">
          References: European Food Safety Authority, Dietary Reference Values (2017) and upper
          levels (2025), for adults; caffeine, its opinion on the safety of caffeine (2015). Sodium:
          World Health Organization. They describe what covers a population, not what you in
          particular need.
        </p>
      {/if}
    {:else if !failed}
      <p class="says">Loading…</p>
    {/if}
  </div>
</div>

<style>
  .nutrients-overlay {
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
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }
  h2 {
    margin: 0;
    font-size: var(--text-lg);
  }
  h3 {
    margin: var(--space-2) 0 0;
    font-size: var(--text-md);
  }
  button {
    font: inherit;
    color: inherit;
  }
  .x {
    flex: none;
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
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }
  li + li {
    border-top: 1px solid var(--border);
  }
  .row {
    width: 100%;
    box-sizing: border-box;
    display: grid;
    grid-template-columns: 1fr auto;
    gap: var(--space-1) var(--space-2);
    align-items: baseline;
    min-height: 2.75rem;
    padding: var(--space-2) 0;
    border: 0;
    background: none;
    text-align: left;
    cursor: pointer;
  }
  .row > :global(:not(.name):not(.val)) {
    grid-column: 1 / -1;
  }
  .val {
    font-variant-numeric: tabular-nums;
  }
  .sub,
  .says,
  .partial {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .over {
    font-size: var(--text-sm);
    color: var(--danger-fg);
  }
  .why {
    padding: 0 0 var(--space-2);
    font-size: var(--text-sm);
  }
  .why p {
    margin: 0 0 var(--space-1);
  }
  .made {
    display: flex;
    justify-content: space-between;
    gap: var(--space-2);
  }
  .failed {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--danger-fg);
  }
  .missing {
    color: var(--text-muted);
  }
  .pick {
    width: 100%;
    min-height: 2.75rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    padding: 0;
    border: 0;
    background: none;
    text-align: left;
    cursor: pointer;
  }
  .pick[aria-pressed='true'] .name {
    font-weight: 600;
  }
  .pick:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .tick {
    color: var(--accent);
  }
  .pin {
    min-height: 2.75rem;
    margin-top: var(--space-1);
    padding: 0 var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    background: none;
    cursor: pointer;
  }
  .pin[aria-pressed='true'] {
    border-color: var(--accent);
  }
  .pin:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
