<script lang="ts">
  // Creating a vault: a name, a folder, and an honest account of what will happen.
  //
  // ONE component, TWO mounts. As the first-run screen it is the whole app, because
  // without a vault there is nothing else to show. As a dialog from the backup panel it
  // makes the second, third, tenth. The only difference is `firstRun`, which changes the
  // words and nothing else — a second implementation would be a second set of promises to
  // keep in sync, and these are exactly the promises that must not drift.
  //
  // A browser cannot open a folder picker for a server's filesystem, and shelling out to
  // zenity would mean the core spawns a process to do its own first run. So: a typed path,
  // and the server answers what is really there on every keystroke.
  import { checkPath, createVault } from './ipc';
  import type { PathCheck, VaultInfo } from './types';
  import { describe, historyNote } from './vaultCheck';

  interface Props {
    /** No vaults exist. The app is not usable until this succeeds. */
    firstRun?: boolean;
    /** From the `ping` heartbeat — a capability, not a guess. `null` = not yet answered. */
    git?: boolean | null;
    oncreated: (vaults: VaultInfo[]) => void;
    oncancel?: () => void;
  }
  let { firstRun = false, git = null, oncreated, oncancel }: Props = $props();

  let name = $state('notes');
  let path = $state('~/notes');
  let check = $state<PathCheck | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);

  const described = $derived(describe(check));
  // The SERVER owns this. Never `described.blocking.length === 0` — that would be the
  // browser holding a second opinion, which is how a button enables and then fails.
  const canCreate = $derived(!!check?.ok && !busy);

  // Debounced like the sidebar's search: a keystroke should not be a round trip, but the
  // answer must feel immediate once you stop.
  let timer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const [n, p] = [name, path];
    clearTimeout(timer);
    if (!p.trim()) {
      check = null;
      return;
    }
    timer = setTimeout(async () => {
      check = await checkPath(n, p).catch(() => null);
    }, 250);
    return () => clearTimeout(timer);
  });

  async function submit() {
    if (!canCreate) return;
    busy = true;
    error = null;
    try {
      oncreated(await createVault(name, path));
    } catch (e) {
      // The server's sentence, verbatim. It is the one that knows what actually happened
      // — including the partial case ("the folder was created, but the list could not be
      // saved"), which we must not smooth over into "couldn't create vault".
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="wrap" class:first={firstRun}>
  <form
    class="card"
    onsubmit={(e) => {
      e.preventDefault();
      void submit();
    }}
  >
    {#if firstRun}
      <h1>Create your first vault</h1>
      <p class="lede">
        A vault is a folder of Markdown files that you own. formicaria keeps your notes in
        it — one note, one file — and nothing else needs to exist for that to work.
      </p>
    {:else}
      <h1>New vault</h1>
      <p class="lede">
        A separate folder, with its own audience. Notes you put here stay here.
      </p>
    {/if}

    <label>
      <span>Name</span>
      <input bind:value={name} placeholder="notes" autocomplete="off" spellcheck="false" />
      <small>What you'll call it here. Anything you like.</small>
    </label>

    <label>
      <span>Folder</span>
      <input
        bind:value={path}
        placeholder="~/notes"
        autocomplete="off"
        spellcheck="false"
        autocapitalize="off"
      />
      <small>Where the files live. Notes go in a <code>notes/</code> folder inside it.</small>
    </label>

    <!-- Everything below is a promise about someone's filesystem. Each line is a fact the
         server reported, never an inference we made. -->
    <ul class="says" aria-live="polite">
      {#each described.blocking as msg (msg)}
        <li class="bad">{msg}</li>
      {/each}
      {#each described.warnings as msg (msg)}
        <li class="warn">{msg}</li>
      {/each}
      {#if check?.ok && described.warnings.length === 0}
        <li class="good">Ready to create.</li>
      {/if}
      {#if git !== null}
        <li class="note">{historyNote(git)}</li>
      {/if}
    </ul>

    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}

    <div class="actions">
      {#if !firstRun && oncancel}
        <button type="button" class="ghost" onclick={oncancel}>Cancel</button>
      {/if}
      <button type="submit" disabled={!canCreate}>
        {busy ? 'Creating…' : 'Create vault'}
      </button>
    </div>
  </form>
</div>

<style>
  .wrap {
    display: flex;
    justify-content: center;
    padding: var(--space-5, 24px);
  }
  .wrap.first {
    align-items: center;
    min-height: 100vh;
  }
  .card {
    width: 100%;
    max-width: 34rem;
    display: flex;
    flex-direction: column;
    gap: var(--space-4, 16px);
  }
  h1 {
    margin: 0;
    font-size: 1.35rem;
  }
  .lede {
    margin: 0;
    color: var(--fg-muted);
    line-height: 1.5;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  label > span {
    font-weight: 600;
    font-size: 0.9rem;
  }
  input {
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-2, 6px);
    background: var(--bg-inset, var(--bg));
    color: var(--fg);
    font: inherit;
  }
  input:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  small,
  code {
    color: var(--fg-muted);
    font-size: 0.85rem;
  }
  .says {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .says li {
    font-size: 0.9rem;
    line-height: 1.45;
    padding-left: 1.4em;
    position: relative;
  }
  .says li::before {
    position: absolute;
    left: 0;
  }
  .bad {
    color: var(--danger, #c0392b);
  }
  .bad::before {
    content: '✗';
  }
  .warn::before {
    content: '⚠';
  }
  .good::before {
    content: '✓';
  }
  .warn,
  .good,
  .note {
    color: var(--fg-muted);
  }
  .note::before {
    content: 'ℹ';
  }
  .error {
    margin: 0;
    padding: 8px 10px;
    border-radius: var(--radius-2, 6px);
    background: color-mix(in srgb, var(--danger, #c0392b) 12%, transparent);
    color: var(--danger, #c0392b);
    font-size: 0.9rem;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  button {
    padding: 8px 14px;
    border: 1px solid transparent;
    border-radius: var(--radius-2, 6px);
    background: var(--accent);
    color: var(--accent-fg, #fff);
    font: inherit;
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .ghost {
    background: transparent;
    border-color: var(--border);
    color: var(--fg);
  }
</style>
