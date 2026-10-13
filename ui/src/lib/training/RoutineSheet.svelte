<script lang="ts">
  // A session kept under a name, and the days of the week it is meant for
  // (`decisions.md#data`, 2026-10-12, *a routine is a session kept under a name*).
  //
  // Two uses, one form: keep today's sets as a new routine, or change one that exists — its name,
  // its days — or remove it. The sets themselves are changed by planning the routine onto a day,
  // changing them there, and keeping that session under the same name.
  import Icon from '../Icon.svelte';
  import Resumed from '../Resumed.svelte';
  import { draftOf, dropDraft, keepDraft } from '../drafts';
  import { createNotesUndoable, deleteNotesUndoable, setPropertiesUndoable } from '../undo.svelte';
  import type { NoteDraft, ObjectMeta, Routine, TotalsDef } from '../types';

  interface Props {
    def: TotalsDef;
    /** The routine to change; `null` to keep `sets` as a new one. */
    routine: Routine | null;
    /** Today's sets, for a new routine. */
    sets: ObjectMeta[];
    /** The routines there are, so a name already taken replaces that routine's sets. */
    routines: Routine[];
    onclose: () => void;
    onwrote: () => void;
  }
  let { def, routine, sets, routines, onclose, onwrote }: Props = $props();

  const DAYS = [
    { key: 'mon', label: 'Mon' },
    { key: 'tue', label: 'Tue' },
    { key: 'wed', label: 'Wed' },
    { key: 'thu', label: 'Thu' },
    { key: 'fri', label: 'Fri' },
    { key: 'sat', label: 'Sat' },
    { key: 'sun', label: 'Sun' },
  ];
  // svelte-ignore state_referenced_locally
  let name = $state(routine?.title ?? '');
  // svelte-ignore state_referenced_locally
  let days = $state<string[]>(routine?.weekdays ?? []);
  let busy = $state(false);
  let failed = $state('');
  // A new routine's name and days are kept on the device until it is saved (`drafts.ts`).
  const DRAFT = 'routine';
  let resumed = $state(false);
  // svelte-ignore state_referenced_locally
  if (!routine) {
    const was = draftOf<{ name: string; days: string[] }>(DRAFT);
    if (was && (was.name || was.days?.length)) {
      name = was.name ?? '';
      days = was.days ?? [];
      resumed = true;
    }
  }
  $effect(() => {
    if (!routine) keepDraft(DRAFT, { name, days });
  });
  const afresh = () => {
    name = '';
    days = [];
    resumed = false;
  };
  const toggle = (d: string) =>
    (days = days.includes(d) ? days.filter((x) => x !== d) : [...days, d]);
  /// In the week's order, whatever order they were pressed in.
  const ordered = $derived(DAYS.map((d) => d.key).filter((d) => days.includes(d)));
  const same = $derived(
    routine ? null : routines.find((r) => r.title.toLowerCase() === name.trim().toLowerCase()),
  );
  const count = $derived(routine ? routine.sets.length : sets.length);
  const ready = $derived(name.trim() !== '' && (routine !== null || sets.length > 0));

  /// A routine's set: what a set is, without the day it was done on or what came of it.
  function asSet(s: ObjectMeta, word: string): NoteDraft {
    const { planned: _p, work_g: _w, session: _s, routine: _r, ...keep } = s.props;
    const props = Object.fromEntries(
      Object.entries(keep).filter(
        (e): e is [string, string | number] => typeof e[1] === 'string' || typeof e[1] === 'number',
      ),
    );
    return { title: s.title ?? '', start: '', props: { ...props, log: def.log, routine: word } };
  }

  async function save() {
    if (busy || !ready) return;
    busy = true;
    failed = '';
    const title = name.trim();
    try {
      if (routine) {
        await setPropertiesUndoable(
          routine.id,
          { title, weekdays: ordered.join(',') },
          `changed the routine “${title}”`,
        );
      } else {
        // The same name again: this session takes the place of that routine's sets.
        if (same)
          await deleteNotesUndoable(
            [same.id, ...same.sets.map((s) => s.id)],
            `replaced the routine “${same.title}”`,
          );
        const word = `routine-${Date.now().toString(36)}`;
        await createNotesUndoable(
          [
            { title, start: '', props: { routine: word, weekdays: ordered.join(',') } },
            ...sets.map((s) => asSet(s, word)),
          ],
          `kept the routine “${title}”`,
        );
        dropDraft(DRAFT);
      }
      onwrote();
      onclose();
    } catch (e) {
      failed = String(e);
    } finally {
      busy = false;
    }
  }
  async function remove() {
    if (busy || !routine) return;
    busy = true;
    try {
      await deleteNotesUndoable(
        [routine.id, ...routine.sets.map((s) => s.id)],
        `removed the routine “${routine.title}”`,
      );
      onwrote();
      onclose();
    } catch (e) {
      failed = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<div class="routine-overlay">
  <button type="button" class="scrim" onclick={onclose} aria-label="close" tabindex="-1"></button>
  <div
    class="sheet"
    role="dialog"
    aria-modal="true"
    aria-label={routine ? 'change a routine' : 'keep a routine'}
  >
    <header>
      <h2>{routine ? 'Routine' : 'Keep this session as a routine'}</h2>
      <button type="button" class="x" onclick={onclose} aria-label="close">
        <Icon name="close" size={18} />
      </button>
    </header>
    <label>
      <span>Name</span>
      <!-- svelte-ignore a11y_autofocus -->
      <input type="text" bind:value={name} autofocus={!routine} autocomplete="off" />
    </label>
    <div class="days" role="group" aria-label="the days it is meant for">
      {#each DAYS as d (d.key)}
        <button
          type="button"
          class="chip"
          class:on={days.includes(d.key)}
          aria-pressed={days.includes(d.key)}
          onclick={() => toggle(d.key)}>{d.label}</button
        >
      {/each}
    </div>
    <p class="says">
      {#if ordered.length}
        On those days it is offered first when you open the session.
      {:else}
        With no day chosen it is still there to plan whenever you like.
      {/if}
      {#if same}
        A routine called “{same.title}” exists: this session will take its place.
      {/if}
    </p>
    <p class="says">
      {count}
      {count === 1 ? 'set' : 'sets'}. To change them, plan the routine, change the session, and keep
      it under the same name.
    </p>
    {#if resumed}<Resumed onafresh={afresh} />{/if}
    {#if failed}<p class="failed" role="alert">{failed}</p>{/if}
    <button type="button" class="go" onclick={save} disabled={busy || !ready}>
      {routine ? 'Save' : 'Keep it'}
    </button>
    {#if routine}
      <button type="button" class="quiet" onclick={remove} disabled={busy}
        >Remove this routine</button
      >
    {/if}
  </div>
</div>

<style>
  .routine-overlay {
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
    gap: var(--space-2);
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
  label {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  input {
    min-height: 3rem;
    padding: 0 var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    font-size: var(--text-lg);
  }
  .days {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  .chip {
    min-width: 3rem;
    min-height: 2.5rem;
    padding: 0 var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    background: none;
    cursor: pointer;
  }
  .chip.on {
    border-color: var(--accent);
    background: var(--accent-subtle);
  }
  .says {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .failed {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--danger-fg);
  }
  .go {
    min-height: 3rem;
    border: 0;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--accent-contrast);
    font-weight: 600;
    cursor: pointer;
  }
  .go:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .quiet {
    min-height: 2.75rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: none;
    color: var(--text-muted);
    cursor: pointer;
  }
</style>
