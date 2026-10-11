<script lang="ts">
  // Writing down a measurement — a weight — for a day. One number for each value the view names,
  // typed in the unit a person uses (kilograms for a value kept in grams).
  import Icon from './Icon.svelte';
  import { createNotesUndoable } from './undo.svelte';
  import { typed, dayLabel } from './units';
  import type { TotalsDef } from './types';

  interface Props {
    def: TotalsDef;
    day: string;
    onclose: () => void;
    onsaved: () => void;
  }
  let { def, day, onclose, onsaved }: Props = $props();

  const fields = $derived(def.metrics.filter((m) => m.role !== 'detail'));
  let typedIn = $state<Record<string, number | string | null | undefined>>({});
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
    const title = fields.map((m) => m.label).join(', ');
    try {
      await createNotesUndoable(
        [{ title, start: day, props: { log: def.log, ...values } }],
        `noted ${title.toLowerCase()}`,
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

<div class="measure-overlay">
  <button type="button" class="scrim" onclick={onclose} aria-label="close" tabindex="-1"></button>
  <div class="sheet" role="dialog" aria-modal="true" aria-label="add a measurement">
    <header>
      <h2>{dayLabel(day)}</h2>
      <button type="button" class="x" onclick={onclose} aria-label="close">
        <Icon name="close" size={18} />
      </button>
    </header>
    {#each fields as m, i (m.key)}
      <label>
        <span>{m.label}{typed(m.key).unit ? ` (${typed(m.key).unit})` : ''}</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input
          type="number"
          inputmode="decimal"
          min="0"
          step="0.1"
          autofocus={i === 0}
          bind:value={typedIn[m.key]}
        />
      </label>
    {/each}
    {#if failed}<p class="failed" role="alert">{failed}</p>{/if}
    <button type="button" class="go" onclick={save} disabled={busy || !ready}>Save</button>
  </div>
</div>

<style>
  .measure-overlay {
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
    font-size: var(--text-xl);
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
