<script lang="ts">
  // **⋯ → History** (`decisions.md` 2026-11-22, later): the note's versions from its vault's history,
  // newest first; one can be read, and *Put this version's text back* writes it as a new change —
  // undoable like any edit, so history is never rewritten.
  import { noteHistory, noteVersion, updateBody } from './ipc';
  import { recordTextEdit } from './undo.svelte';
  import { clockText, formatDay } from './appTime';

  let {
    id,
    title,
    current,
    base,
    onclose,
    onrestored,
  }: {
    id: string;
    title: string;
    /** The text as it is now. */
    current: string;
    /** Its version (the server's lost-update guard compares this, a hash of the body). */
    base: string;
    onclose: () => void;
    onrestored: () => void;
  } = $props();

  let versions = $state<{ commit: string; author: string; time: string }[] | null>(null);
  let chosen = $state<string | null>(null);
  let body = $state<string | null>(null);
  let failed = $state('');

  $effect(() => {
    void noteHistory(id)
      .then((v) => (versions = v))
      .catch((e) => {
        failed = String(e);
        versions = [];
      });
  });

  async function show(commit: string) {
    chosen = commit;
    body = null;
    try {
      body = (await noteVersion(id, commit)).body;
    } catch (e) {
      failed = String(e);
    }
  }

  async function putBack() {
    if (body === null) return;
    try {
      await updateBody(id, body, base);
      recordTextEdit(id, title, current, body);
      onrestored();
      onclose();
    } catch (e) {
      failed = String(e);
    }
  }

  /** On the app's clock (one clock, `appTime.ts`), not the browser's. */
  const when = (iso: string) => `${formatDay(iso)} ${clockText(iso)}`;
</script>

<div class="history" role="dialog" aria-label="history of this note">
  <header>
    <h2>History</h2>
    <button type="button" aria-label="close history" onclick={onclose}>✕</button>
  </header>
  {#if failed}<p class="error" role="alert">{failed}</p>{/if}
  {#if versions === null}
    <p class="muted">Reading the history…</p>
  {:else if !versions.length}
    <p class="muted">
      No history yet. A note's versions are kept once its notebook is backed up with history (git).
    </p>
  {:else}
    <div class="cols">
      <ol class="versions" aria-label="versions">
        {#each versions as v, i (v.commit)}
          <li>
            <button
              type="button"
              class:on={chosen === v.commit}
              aria-current={chosen === v.commit ? 'true' : undefined}
              onclick={() => void show(v.commit)}
              >{when(v.time)}{i === 0 ? ' · latest' : ''}<span class="who">{v.author}</span></button
            >
          </li>
        {/each}
      </ol>
      <div class="version">
        {#if chosen && body !== null}
          <pre aria-label="this version's text">{body}</pre>
          <button
            type="button"
            class="put-back"
            onclick={() => void putBack()}
            disabled={body === current}>Put this version's text back</button
          >
        {:else if chosen}
          <p class="muted">Reading…</p>
        {:else}
          <p class="muted">Choose a version to read it.</p>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .history {
    position: fixed;
    inset: 6dvh 6vw;
    z-index: 120;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
    display: flex;
    flex-direction: column;
    padding: var(--space-3) var(--space-4);
    overflow: hidden;
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
  header button {
    border: none;
    background: transparent;
    cursor: pointer;
    font-size: 1.1rem;
  }
  .cols {
    display: grid;
    grid-template-columns: minmax(12rem, 18rem) 1fr;
    gap: var(--space-4);
    min-height: 0;
    flex: 1;
    margin-top: var(--space-3);
  }
  .versions {
    list-style: none;
    margin: 0;
    padding: 0;
    overflow: auto;
  }
  .versions button {
    width: 100%;
    text-align: left;
    border: 1px solid transparent;
    background: none;
    padding: var(--space-2);
    border-radius: var(--radius-md);
    cursor: pointer;
    font: inherit;
    display: flex;
    flex-direction: column;
  }
  .versions button.on {
    border-color: var(--accent);
    background: var(--card-bg);
  }
  .who {
    color: var(--text-muted);
    font-size: var(--text-xs);
  }
  .version {
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  pre {
    white-space: pre-wrap;
    background: var(--card-bg);
    padding: var(--space-3);
    border-radius: var(--radius-md);
    margin: 0;
    flex: 1;
  }
  .put-back {
    align-self: flex-start;
    font: inherit;
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    border: 1px solid var(--accent);
    background: var(--accent);
    color: var(--accent-fg, #fff);
    cursor: pointer;
  }
  .muted {
    color: var(--text-muted);
  }
  .error {
    color: var(--danger-fg);
  }
  @media (max-width: 40rem) {
    .history {
      inset: 0;
      border-radius: 0;
    }
    .cols {
      grid-template-columns: 1fr;
    }
  }
</style>
