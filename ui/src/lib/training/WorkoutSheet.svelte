<script lang="ts">
  // Planning an exercise for a session — so many sets of so many reps at a load — or changing one
  // set already there.
  //
  // One note per set (`decisions.md#data`, 2026-10-11), each planned until it is ticked. The name
  // of an exercise is whatever the person types; the ones used before are offered, with the
  // numbers they had last time already filled in.
  import { untrack } from 'svelte';
  import Icon from '../Icon.svelte';
  import { createNotesUndoable, setPropertiesUndoable, deleteUndoable } from '../undo.svelte';
  import { whole } from '../units';
  import type { NoteDraft, ObjectMeta, RecentItem, TotalsDef } from '../types';

  interface Props {
    def: TotalsDef;
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
  let { def, day, set = null, last, have, onclose, onwrote }: Props = $props();

  const KG = 1000;
  let name = $state(untrack(() => String(set?.props.exercise ?? set?.title ?? '')));
  // **Nothing here is pre-filled with a number of our own.** How many sets, how many reps and how
  // heavy is advice, and advice needs a source (`decisions.md#data`, 2026-10-11, *nothing is
  // invented*). The fields start from the set being changed, or from what this exercise was last
  // time once it is chosen — and otherwise empty.
  let sets = $state(1);
  let reps = $state(untrack(() => whole(set?.props.reps) ?? 0));
  let kg = $state(untrack(() => (whole(set?.props.load_g) ?? 0) / KG));
  let busy = $state(false);
  let failed = $state('');

  /// Choosing an exercise done before brings back what it was last time.
  function choose(r: RecentItem) {
    name = r.title;
    reps = whole(r.props.reps) ?? reps;
    kg = (whole(r.props.load_g) ?? kg * KG) / KG;
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
        },
      }));
      await createNotesUndoable(drafts, `planned ${exercise}`);
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
        { reps, load_g, ...(set.props.planned ? {} : { work_g: reps * load_g }) },
        `changed a set of ${set.title ?? 'an exercise'}`,
      );
      onwrote();
      onclose();
    });

  const remove = () =>
    write(async () => {
      if (!set) return;
      const title = `set ${whole(set.props.set) ?? ''} of ${set.title ?? 'an exercise'}`;
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
    aria-label={set ? 'change a set' : 'add an exercise'}
  >
    <header>
      <h2>{set ? `${name} · set ${whole(set.props.set) ?? ''}` : 'Add an exercise'}</h2>
      <button type="button" class="x" onclick={onclose} aria-label="close">
        <Icon name="close" size={18} />
      </button>
    </header>

    {#if !set}
      <label class="field">
        <span>Exercise</span>
        <input type="text" bind:value={name} autocomplete="off" />
      </label>
      {#if last.length}
        <div class="chips">
          {#each last.slice(0, 8) as r (r.title)}
            <button type="button" class="chip" class:on={r.title === name} onclick={() => choose(r)}
              >{r.title}</button
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

    {#if !set}{@render stepper(
        'Sets',
        sets,
        1,
        1,
        (v) => (sets = Math.min(Math.round(v), 12)),
      )}{/if}
    {@render stepper('Reps', reps, 1, 0, (v) => (reps = Math.round(v)))}
    {@render stepper('Load (kg)', kg, 2.5, 0, (v) => (kg = v))}

    {#if failed}<p class="failed" role="alert">{failed}</p>{/if}

    {#if set}
      <div class="pair">
        <button type="button" class="quiet" onclick={remove} disabled={busy}>Remove</button>
        <button type="button" class="go" onclick={save} disabled={busy || !ready}>Save</button>
      </div>
    {:else}
      <button type="button" class="go" onclick={add} disabled={busy || !ready}>
        Add {sets} × {reps}
      </button>
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
</style>
