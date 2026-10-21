<script lang="ts">
  // **Track something** (`decisions.md#ui`, 2026-10-22): what the Health group's plus opens. A
  // starter, or one thing the person names, is written as keys into their Health template — a
  // note — so what they track is in their notebook and nowhere else.
  import Icon from '../Icon.svelte';
  import ManualLink from '../ManualLink.svelte';
  import { createNotesUndoable, setPropertiesUndoable } from '../undo.svelte';
  import { KINDS, STARTERS, TEMPLATE, UNITS, stemOf, type Starter } from './starters';

  interface Props {
    /** The Health template's id, or null when the person tracks nothing yet. */
    template: string | null;
    /** The keys already tracked. */
    known: string[];
    onclose: () => void;
    onsaved: () => void;
  }
  let { template, known, onclose, onsaved }: Props = $props();

  let name = $state('');
  let kind = $state(KINDS[0].kind);
  let unit = $state(UNITS[0].unit);
  let busy = $state(false);
  let failed = $state('');

  const stem = $derived(stemOf(name));
  const key = $derived.by(() => {
    if (!stem) return '';
    const k = KINDS.find((x) => x.kind === kind)!;
    return k.kind === 'amount' ? `${stem}_${unit}` : `${stem}${k.suffix}`;
  });
  const taken = $derived(key !== '' && known.includes(key));

  /** Keys a starter would add that are not tracked already. */
  const missing = (s: Starter) => s.keys.filter((k) => !known.includes(k));

  async function add(keys: string[], label: string) {
    if (busy || keys.length === 0) return;
    busy = true;
    failed = '';
    const props = Object.fromEntries(keys.map((k) => [k, 0]));
    try {
      if (template) await setPropertiesUndoable(template, props, label);
      else
        await createNotesUndoable(
          [
            {
              title: TEMPLATE.title,
              body: TEMPLATE.body,
              tags: TEMPLATE.tags,
              props: { group: TEMPLATE.group, ...props },
            },
          ],
          label,
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

<div class="track-overlay">
  <button type="button" class="scrim" onclick={onclose} aria-label="close" tabindex="-1"></button>
  <div class="sheet" role="dialog" aria-modal="true" aria-label="track something">
    <header>
      <h2>Track something</h2>
      <button type="button" class="x" onclick={onclose} aria-label="close">
        <Icon name="close" size={18} />
      </button>
    </header>

    <section aria-label="starters">
      {#each STARTERS as s (s.name)}
        {@const left = missing(s)}
        <button
          type="button"
          class="starter"
          disabled={busy || left.length === 0}
          onclick={() => add(left, `started tracking ${s.name.toLowerCase()}`)}
        >
          <strong>{s.name}</strong>
          <span>{left.length === 0 ? 'Already tracked' : s.about}</span>
          <small>{s.source}</small>
          {#if s.before}<small class="before">{s.before}</small>{/if}
        </button>
      {/each}
    </section>

    <ManualLink
      page="user/health.html"
      section="keeping-cycle-notes-to-yourself"
      label="Keeping cycle notes to yourself"
    />

    <section class="own" aria-label="your own">
      <h3>Or name it yourself</h3>
      <label>
        <span>What</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input type="text" bind:value={name} placeholder="Bedding washed" autofocus />
      </label>
      <label>
        <span>Recorded as</span>
        <select bind:value={kind}>
          {#each KINDS as k (k.kind)}<option value={k.kind}>{k.label}</option>{/each}
        </select>
      </label>
      {#if kind === 'amount'}
        <label>
          <span>In</span>
          <select bind:value={unit}>
            {#each UNITS as u (u.unit)}<option value={u.unit}>{u.label}</option>{/each}
          </select>
        </label>
      {/if}
      {#if name.trim() && !stem}
        <p class="note">Use letters from a to z for now; other scripts are not supported yet.</p>
      {:else if key}
        <p class="note">Kept in your Health diary note as <code>{key}</code>.</p>
      {/if}
      {#if taken}<p class="note">You track this already.</p>{/if}
      {#if failed}<p class="failed" role="alert">{failed}</p>{/if}
      <button
        type="button"
        class="go"
        disabled={busy || !key || taken}
        onclick={() => add([key], `started tracking ${name.trim().toLowerCase()}`)}>Track it</button
      >
    </section>
  </div>
</div>

<style>
  .track-overlay {
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
    gap: var(--space-4);
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
  h3 {
    margin: 0;
    font-size: var(--text-md);
  }
  button,
  input,
  select {
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
  section {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .starter {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-1);
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    text-align: left;
    cursor: pointer;
  }
  .starter:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .starter span {
    font-size: var(--text-sm);
  }
  .starter .before {
    color: var(--text);
  }
  .starter small,
  .note {
    font-size: var(--text-xs);
    color: var(--text-muted);
  }
  .note {
    margin: 0;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  input,
  select {
    min-height: 3rem;
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
