<script lang="ts">
  // Every vitamin and mineral of a day, each beside the figure a named body gives for it
  // (`nutrients.rs`; `decisions.md#data`, 2026-10-12). The day screen names four; this is the rest.
  //
  // A row says three things and no more: what the day came to, the reference where there is one
  // figure for this person, and — pressed — which entries it came from and, for each entry without
  // it, why (2026-10-29: "No entry of this day carries it" under choline, and nothing about the eggs,
  // read as the app failing). Entries logged before the food list had values it has now are offered
  // an update, on a press: an entry is a snapshot, and only the person changes it.
  import { onMount } from 'svelte';
  import Icon from '../Icon.svelte';
  import Meter from '../charts/Meter.svelte';
  import { foodGet, logNutrients, setProperties } from '../ipc';
  import { dayLabel, showWith } from '../units';
  import { borrowedOf, valueOf } from './provenance';
  import { freshValues, fromList, whyNot } from './refresh';
  import type { FoodDetail, NutrientRow, Nutrients, ObjectMeta } from '../types';

  interface Props {
    view: string;
    day: string;
    onclose: () => void;
    /** Entries were brought up to date: the day behind should read them again. */
    onwrote?: () => void;
  }
  let { view, day, onclose, onwrote }: Props = $props();

  let got = $state<Nutrients | null>(null);
  let failed = $state('');
  let opened = $state<string | null>(null);
  /// Each list entry's food as the list has it now, by entry id; `null` where it is not in the list.
  let foods = $state<Record<string, FoodDetail | null>>({});
  let updating = $state(false);
  async function load() {
    got = await logNutrients(view, day);
    const listed = got.entries.filter((e) => fromList(e.props));
    const read = await Promise.all(
      listed.map(
        async (e) => [e.id, await foodGet(String(e.props.source)).catch(() => null)] as const,
      ),
    );
    foods = Object.fromEntries(read);
  }
  /// The entries the list could complete, with what it would add.
  const stale = $derived(
    (got?.entries ?? []).flatMap((e) => {
      const f = foods[e.id];
      const add = f ? freshValues(e.props, f) : null;
      return add ? [{ e, add }] : [];
    }),
  );
  async function update() {
    updating = true;
    try {
      for (const { e, add } of stale) await setProperties(e.id, add);
      await load();
      onwrote?.();
    } catch (e) {
      failed = String(e);
    } finally {
      updating = false;
    }
  }
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
      .map((e) => ({ e, why: whyNot(e.props, r.key, fromList(e.props) ? foods[e.id] : null) }));
  const share = (r: NutrientRow) => (r.reference ? Math.round((r.sum / r.reference) * 100) : null);
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<div class="nutrients-overlay">
  <button type="button" class="scrim" onclick={onclose} aria-label="close" tabindex="-1"></button>
  <div class="sheet" role="dialog" aria-modal="true" aria-label="all nutrients">
    <header>
      <h2>Nutrients · {dayLabel(day)}</h2>
      <button type="button" class="x" onclick={onclose} aria-label="close">
        <Icon name="close" size={18} />
      </button>
    </header>
    {#if failed}<p class="failed" role="alert">{failed}</p>{/if}
    {#if got}
      {#if got.of === 0}
        <p class="says">Nothing is logged for this day.</p>
      {:else}
        {#if stale.length}
          <div class="stale" role="status">
            <p>
              {stale.length === 1 ? 'One entry was' : `${stale.length} entries were`} logged before the
              food list had some of the values it has now (choline, for instance).
            </p>
            <button type="button" onclick={update} disabled={updating}>
              {updating ? 'Updating…' : stale.length === 1 ? 'Update it' : 'Update them'}
            </button>
          </div>
        {/if}
        {#if got.needs_sex}
          <p class="says">
            Some references differ for men and women and are left out until you say which, under
            Diet, in About you.
          </p>
        {/if}
        <ul>
          {#each got.rows as r (r.key)}
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
                  >{showWith(r.key, r.sum, r.reference ?? r.upper ?? r.sum)}{#if r.reference}<span
                      class="sub">&nbsp;/ {showWith(r.key, r.reference)}</span
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
                  <span class="over">above the upper level of {showWith(r.key, r.upper)}</span>
                {:else if r.upper && !r.reference}
                  <span class="sub">limit {showWith(r.key, r.upper)}</span>
                {/if}
                {#if r.n < got.of}
                  <span class="partial">from {r.n} of {got.of} entries</span>
                {/if}
              </button>
              {#if opened === r.key}
                <div class="why">
                  {#each from(r) as e (e.id)}
                    <p class="made">
                      <span
                        >{e.title ?? 'Untitled'}{#if borrowedOf(e.props)[r.key]}<span class="sub">
                            · borrowed from {borrowedOf(e.props)[r.key]}</span
                          >{/if}</span
                      >
                      <span>{showWith(r.key, valueOf(e.props, r.key) ?? 0)}</span>
                    </p>
                  {/each}
                  {#each without(r) as { e, why } (e.id)}
                    <p class="made missing">
                      <span
                        >{e.title ?? 'Untitled'}{#if why}<span class="sub">
                            · {why}</span
                          >{/if}</span
                      >
                      <span>—</span>
                    </p>
                  {/each}
                  {#if r.kind && KIND[r.kind]}<p class="sub">Reference: {KIND[r.kind]}.</p>{/if}
                  {#if r.note}<p class="sub">{r.note}.</p>{/if}
                </div>
              {/if}
            </li>
          {/each}
        </ul>
        <p class="says">
          References: European Food Safety Authority, Dietary Reference Values (2017) and upper
          levels (2025), for adults. Sodium: World Health Organization. They describe what covers a
          population, not what you in particular need.
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
  .stale {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    margin: 0 0 var(--space-2);
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-2, 6px);
    background: var(--surface);
  }
  .stale p {
    margin: 0;
  }
  .stale button {
    flex: none;
    min-height: 2.75rem;
  }
  .missing {
    color: var(--text-muted);
  }
</style>
