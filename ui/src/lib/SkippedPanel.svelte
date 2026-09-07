<script lang="ts">
  // The notes the vault could not read — a place, not a toast.
  //
  // A note whose frontmatter will not parse is absent from *every* view: the board, the
  // agenda, search, all of it. That is deliberate (see `docs/context/decisions.md`) —
  // the alternative, letting one side's field win, would resolve a real disagreement by
  // fiat and push it silently. But "absent and named in a notification" is only half an
  // answer: the notification scrolls away and leaves you with a filename and no way to
  // act on it. This is the other half.
  //
  // The one action available is **open it in the OS editor**, and that is not a
  // limitation to apologise for: the note does not parse, so no editor of ours can load
  // it. The conflict markers are plain text in a plain file, and the fix is to delete the
  // lines you do not want.
  //
  // The backend resolves the path from its own current skipped set, so a panel left open
  // across a fix fails closed rather than opening the wrong thing.
  import { openSkipped, readSkipped, resolveSkipped, type SkippedNote } from './ipc';
  import { isRemote } from './remote';

  let { skipped, onclose }: { skipped: SkippedNote[]; onclose: () => void } = $props();

  // Per-row, because one note failing to open says nothing about the others.
  let failed = $state<Record<string, string>>({});

  // `\0` as the escape, not a literal NUL byte in the source. Same value, but a raw NUL made
  // this file `data` rather than text, and **grep skips a binary file silently** — so every
  // `ci/checks.sh` guard that sweeps `ui/src` was passing over it without saying so. The
  // overlay-height guard added 2026-09-01 found the `76vh` here only after this was escaped.
  const key = (s: SkippedNote) => `${s.vault}\0${s.name}`;

  // In-app raw editor state, keyed per note. The UI is the only way in (and the phone has no
  // OS editor), so fixing a conflict here is the real path, not a fallback. `editing` holds the
  // key of the note being fixed; `draft` its raw text.
  let editing = $state<string | null>(null);
  let draft = $state('');
  let saving = $state(false);
  let saveNote = $state('');

  // A conflict is gone only once the file parses again. Reflect that live as the user edits, so
  // "delete the markers" gives immediate feedback rather than save-and-pray.
  const stillConflicted = $derived(/^(<{7}|={7}|>{7}|\|{7})/m.test(draft));

  async function startEdit(s: SkippedNote) {
    saveNote = '';
    try {
      const { text } = await readSkipped(s.vault, s.name);
      draft = text;
      editing = key(s);
      delete failed[key(s)];
    } catch (e) {
      failed[key(s)] = e instanceof Error ? e.message : String(e);
    }
  }

  function cancelEdit() {
    editing = null;
    draft = '';
    saveNote = '';
  }

  async function save(s: SkippedNote) {
    saving = true;
    saveNote = '';
    try {
      const { parses } = await resolveSkipped(s.vault, s.name, draft);
      if (parses) {
        editing = null; // resolved — it drops off this list on the next poll
        draft = '';
      } else {
        saveNote = 'Saved, but it still has conflict markers — remove them and save again.';
      }
      delete failed[key(s)];
    } catch (e) {
      failed[key(s)] = e instanceof Error ? e.message : String(e);
    } finally {
      saving = false;
    }
  }

  // Grouped by vault: two vaults can hold the same filename, and which audience a broken
  // note belongs to is the first thing you need to know to decide whether it is yours.
  const byVault = $derived(
    Object.entries(
      skipped.reduce<Record<string, SkippedNote[]>>((acc, s) => {
        (acc[s.vault] ??= []).push(s);
        return acc;
      }, {}),
    ),
  );

  async function open(s: SkippedNote) {
    try {
      await openSkipped(s.vault, s.name);
      delete failed[key(s)];
    } catch (e) {
      failed[key(s)] = e instanceof Error ? e.message : String(e);
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose();
  }
</script>

<svelte:window {onkeydown} />

<div class="skipped-overlay">
  <button class="skipped-backdrop" aria-label="close" onclick={onclose}></button>
  <div class="panel" role="dialog" aria-label="notes that could not be read">
    <div class="panel-head">
      <h2>Notes that could not be read</h2>
    </div>

    <p class="lede">
      These files are on disk but their frontmatter does not parse, so they are missing from every
      view until that is fixed. The usual cause is a merge where two people changed the same field —
      the disagreement is kept rather than resolved by fiat, and the conflict markers landed inside
      the YAML.
    </p>

    {#each byVault as [vault, notes] (vault)}
      <div class="vault">
        <h3>{vault}</h3>
        {#each notes as s (s.name)}
          <div class="note">
            <div class="line">
              <code>{s.name}</code>
              <div class="row-actions">
                <button class="primary" onclick={() => startEdit(s)}>Fix here</button>
                <!-- Desktop only, literally: it opens an editor on the computer. "Fix here"
                     beside it is the in-app path, which is why hiding this loses nothing. -->
                {#if !isRemote()}
                  <button
                    class="ghost"
                    onclick={() => open(s)}
                    title="Open in your computer's editor (desktop only)"
                  >
                    Open in OS editor
                  </button>
                {/if}
              </div>
            </div>
            <p class="reason">{s.reason}</p>

            {#if editing === key(s)}
              <div class="editor">
                <textarea
                  bind:value={draft}
                  spellcheck="false"
                  rows="14"
                  aria-label="raw text of {s.name}"></textarea>
                <div class="editor-foot">
                  <span
                    class="marker-state"
                    class:bad={stillConflicted}
                    class:ok={!stillConflicted}
                  >
                    {stillConflicted ? '● still has conflict markers' : '● no conflict markers'}
                  </span>
                  <span class="spacer"></span>
                  <button class="ghost" onclick={cancelEdit} disabled={saving}>Cancel</button>
                  <button class="primary" onclick={() => save(s)} disabled={saving}>
                    {saving ? 'Saving…' : 'Save'}
                  </button>
                </div>
                {#if saveNote}
                  <p class="save-note">{saveNote}</p>
                {/if}
              </div>
            {/if}

            {#if failed[key(s)]}
              <p class="failed">{failed[key(s)]}</p>
            {/if}
          </div>
        {/each}
      </div>
    {/each}

    <p class="hint">
      Press <strong>Fix here</strong> to edit the raw file: delete the
      <code>&lt;&lt;&lt;&lt;&lt;&lt;&lt;</code>, <code>=======</code> and
      <code>&gt;&gt;&gt;&gt;&gt;&gt;&gt;</code> lines, keep the value you want, and save. The note reappears
      on the next poll.
    </p>

    <div class="actions">
      <button onclick={onclose}>Close</button>
    </div>
  </div>
</div>

<style>
  .skipped-overlay {
    position: fixed;
    inset: 0;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    /* `border-box` + an explicit `100dvh`, so the panel below can simply say
       `max-height: 100%` and inherit the arithmetic instead of restating it.
       Longhands, never the `padding` shorthand: a narrower rule added later would reset all
       four sides and silently discard the insets, which is the mistake `ci/checks.sh`
       already guards against on `.topbar`. */
    box-sizing: border-box;
    height: 100dvh;
    padding-top: calc(var(--overlay-inset) + var(--safe-top));
    padding-bottom: var(--safe-bottom);
    z-index: 80;
  }
  /* The navigation-bar floor, on the overlay so the panel's *box* clears the bar and not just
     its last row. `--bar-floor` rather than a fourth hand-copied `3.25rem`. */
  @media (pointer: coarse) {
    .skipped-overlay {
      padding-bottom: max(var(--safe-bottom), var(--bar-floor));
    }
  }
  .skipped-backdrop {
    position: fixed;
    inset: 0;
    border: none;
    background: rgb(0 0 0 / 0.45);
    cursor: default;
  }
  .panel {
    position: relative;
    width: min(34rem, 92vw);
    /* `100%` of the overlay's content box, which is already the visible viewport less the
       top offset and both window insets. */
    max-height: 100%;
    overflow-y: auto;
    overscroll-behavior: contain;
    box-sizing: border-box;
    padding: var(--space-5);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    background: var(--surface-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
  }
  .panel-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
  }
  h2 {
    margin: 0;
    font-size: var(--text-md);
    color: var(--text);
  }
  h3 {
    margin: 0 0 var(--space-2);
    font-size: var(--text-sm);
    color: var(--text-muted);
    font-weight: 600;
  }
  .lede,
  .hint {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
    line-height: 1.5;
  }
  .vault {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .note {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: var(--space-2) 0;
    border-top: 1px solid var(--border);
  }
  .line {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }
  .reason {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .row-actions {
    display: flex;
    gap: var(--space-2);
    flex-shrink: 0;
  }
  button.primary {
    background: var(--accent, #6639ba);
    border-color: var(--accent, #6639ba);
    color: #fff;
  }
  button.ghost {
    color: var(--text-muted);
  }
  .editor {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }
  .editor textarea {
    width: 100%;
    box-sizing: border-box;
    resize: vertical;
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 0.82rem;
    line-height: 1.5;
    padding: var(--space-2);
    background: var(--surface-2, rgba(127, 127, 127, 0.08));
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: var(--radius-2, 6px);
    white-space: pre;
    overflow-wrap: normal;
    overflow-x: auto;
  }
  .editor-foot {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .spacer {
    flex: 1;
  }
  .marker-state {
    font-size: var(--text-sm);
  }
  .marker-state.bad {
    color: var(--danger, #c0392b);
  }
  .marker-state.ok {
    color: var(--success, #1a7f37);
  }
  .save-note {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--danger, #c0392b);
  }
  .failed {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--danger, #c0392b);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
  }
  button {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--fg);
    padding: 4px 10px;
    border-radius: var(--radius-2, 6px);
    font-size: 0.85rem;
    cursor: pointer;
  }
  .skipped-backdrop {
    padding: 0;
  }
</style>
