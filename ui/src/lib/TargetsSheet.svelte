<script lang="ts">
  // The targets a day is measured against: one number for each value the view draws large.
  //
  // Saving makes a note — an ordinary one, carrying `program: <log>` and today's date — and the
  // day screens use the latest such note that had started by the day they show. So changing a
  // target does not rewrite the past: yesterday is still measured against yesterday's.
  import { untrack } from 'svelte';
  import Icon from './Icon.svelte';
  import { createNotesUndoable } from './undo.svelte';
  import { typed } from './units';
  import type { TotalsDef } from './types';

  interface Props {
    def: TotalsDef;
    /** What the note is called: the group's name, so it is findable on the agenda. */
    name: string;
    /** The targets in force now, by metric key. */
    current: Record<string, number>;
    today: string;
    onclose: () => void;
    onsaved: () => void;
  }
  let { def, name, current, today, onclose, onsaved }: Props = $props();

  const fields = $derived(def.metrics.filter((m) => m.role !== 'detail'));
  const shownAs = (key: string) => {
    const v = current[key];
    return v === undefined ? '' : String(Math.round((v / typed(key).per) * 10) / 10);
  };
  // Filled once, from the targets in force when the form opened.
  let typedIn = $state<Record<string, number | string | null | undefined>>(
    untrack(() => Object.fromEntries(def.metrics.map((m) => [m.key, shownAs(m.key)]))),
  );
  /** The program's last day, if it has one. After it the earlier targets are in force again. */
  let until = $state('');
  let busy = $state(false);
  let failed = $state('');

  const values = $derived.by(() => {
    const out: Record<string, number> = {};
    for (const m of fields) {
      const raw = typedIn[m.key];
      if (raw === '' || raw === null || raw === undefined) continue;
      const n = Number(raw);
      if (Number.isFinite(n) && n > 0) out[m.key] = Math.round(n * typed(m.key).per);
    }
    return out;
  });
  const ready = $derived(Object.keys(values).length > 0);

  async function save() {
    if (busy || !ready) return;
    busy = true;
    failed = '';
    try {
      await createNotesUndoable(
        [
          {
            title: `${name} targets`,
            start: today,
            tags: ['program'],
            props: { program: def.log, ...values, ...(until ? { due: until } : {}) },
          },
        ],
        `set ${name.toLowerCase()} targets`,
      );
      onsaved();
      onclose();
    } catch (e) {
      failed = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<div class="targets-overlay">
  <button type="button" class="scrim" onclick={onclose} aria-label="close" tabindex="-1"></button>
  <div class="sheet" role="dialog" aria-modal="true" aria-label={`${name} targets`}>
    <header>
      <h2>Targets</h2>
      <button type="button" class="x" onclick={onclose} aria-label="close">
        <Icon name="close" size={18} />
      </button>
    </header>
    <p class="lede">For each day from today. Earlier days keep the targets they had.</p>
    {#each fields as m (m.key)}
      <label>
        <span>{m.label}{typed(m.key).unit ? ` (${typed(m.key).unit})` : ''}</span>
        <input type="number" inputmode="decimal" min="0" bind:value={typedIn[m.key]} />
      </label>
    {/each}
    <label>
      <span>Until (optional)</span>
      <input type="date" min={today} bind:value={until} />
    </label>
    {#if failed}<p class="failed" role="alert">{failed}</p>{/if}
    <button type="button" class="go" onclick={save} disabled={busy || !ready}>Save targets</button>
  </div>
</div>

<style>
  .targets-overlay {
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
  .lede {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
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
  label {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  input {
    min-height: 2.75rem;
    padding: 0 var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
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
</style>
