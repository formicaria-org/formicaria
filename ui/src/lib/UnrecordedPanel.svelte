<script lang="ts">
  /// **What git does not have, and in which way.**
  ///
  /// The count alone was not a diagnosis. The owner's phone said *"146 not in history"* and that could
  /// have meant 146 notes existing nowhere else — app-private storage is erased by an uninstall, and
  /// `blobs/` never travels with a push — or 146 notes something was needlessly rewriting. Opposite
  /// urgencies, one number.
  ///
  /// So this panel leads with the split, then names enough notes to recognise a pattern (all created in
  /// one minute? all the same size?). It exists because **a phone has no other channel**: Rust's stdout
  /// is not routed to logcat, the WebView forwards no `console.*`, and MIUI suppresses our own tag — so
  /// anything the user must be able to report has to be on screen. Same reasoning, and same shape, as
  /// `SkippedPanel`.

  import type { Unrecorded } from './types';
  import { labelFor } from './vaultLabels.svelte';

  interface Props {
    unrecorded: Unrecorded[];
    onrecord: (vault: string) => Promise<void> | void;
    onclose: () => void;
  }
  let { unrecorded, onrecord, onclose }: Props = $props();

  let busy = $state<string | null>(null);
  async function record(vault: string) {
    busy = vault;
    try {
      await onrecord(vault);
    } finally {
      busy = null;
    }
  }

  /// One sentence per kind, because what to *do* differs. Written as consequences, not categories.
  const meaning: Record<string, string> = {
    new: 'Never committed. If this vault has no remote, these exist in one place only.',
    modified: 'Changed since they were committed. A large number here suggests something is rewriting notes.',
    deleted: 'Deleted here, and the deletion has not been recorded.',
  };

  const kinds = ['new', 'modified', 'deleted'] as const;
  const countOf = (u: Unrecorded, k: string) =>
    k === 'new' ? u.new : k === 'modified' ? u.modified : u.deleted;

  function size(bytes: number | null): string {
    if (bytes == null) return '';
    return bytes < 1024 ? `${bytes} B` : `${Math.round(bytes / 1024)} kB`;
  }
  /// Date and minute only — the pattern is what matters ("all within one minute"), not the second.
  function when(stamp: string | null): string {
    if (!stamp) return '';
    return stamp.slice(0, 16).replace('T', ' ');
  }
</script>

<section class="panel" aria-label="Notes not in history">
  <header>
    <h2>Not in history</h2>
    <button type="button" class="close" onclick={onclose} aria-label="Close">✕</button>
  </header>

  {#each unrecorded as u (u.vault)}
    <article>
      <h3>{labelFor(u.vault)}</h3>
      <p class="total">
        {u.count} note{u.count === 1 ? '' : 's'} git does not have.
      </p>
      <ul class="kinds">
        {#each kinds as k (k)}
          {#if countOf(u, k) > 0}
            <li><strong>{countOf(u, k)} {k}</strong> — {meaning[k]}</li>
          {/if}
        {/each}
      </ul>

      <!-- The evidence, bounded. Enough rows to see a pattern; the counts above are the whole truth. -->
      {#if u.notes.length}
        <ul class="notes">
          {#each u.notes as n (n.path)}
            <li>
              <span class="kind" data-kind={n.kind}>{n.kind}</span>
              <span class="role">{n.role}</span>
              <span class="title">{n.title ?? n.id}</span>
              <!-- **The duplicate count is the diagnosis.** "147 notes" says nothing; "one note
                   written 138 times" names a loop. -->
              {#if n.copies > 1}<span class="copies">×{n.copies}</span>{/if}
              <span class="meta">{size(n.bytes)}{n.bytes != null && n.modified ? ' · ' : ''}{when(n.modified)}</span>
            </li>
          {/each}
        </ul>
        {#if u.notes.length < u.count}
          <p class="more">Showing {u.notes.length} of {u.count}.</p>
        {/if}
      {/if}

      <button type="button" disabled={busy === u.vault} onclick={() => record(u.vault)}>
        {busy === u.vault ? 'Recording…' : `Record all ${u.count} in history`}
      </button>
      <!-- Said here because a commit is not a backup, and on a phone the vault may be the only copy
           until it is pushed. -->
      <p class="after">Then back up, to send them to a remote.</p>
    </article>
  {/each}

  {#if !unrecorded.length}
    <p class="total">Nothing outstanding — git has every note.</p>
  {/if}
</section>

<style>
  .panel {
    position: fixed;
    inset: 0;
    z-index: 40;
    overflow: auto;
    /* Respect the window insets: this is a full-screen surface, and `index.html` asks for
       `viewport-fit=cover`, so without these the content paints under the status bar and the
       navigation bar. */
    padding: calc(1.25rem + var(--safe-top)) calc(1.25rem + var(--safe-right))
      calc(1.25rem + var(--safe-bottom)) calc(1.25rem + var(--safe-left));
    background: var(--bg);
    color: var(--text);
  }
  /* **The navigation bar ate the button.** `env(safe-area-inset-*)` resolves to 0 in the Android
     WebView (wry does not forward the window insets — see `app.css`), and the global `--safe-bottom`
     floor is 0.5rem, which clears a gesture pill but not a 3-button navigation bar. Observed on the
     owner's phone: "Record all 147 in history" was half under the system bar, and the line below it
     was invisible. Floored locally rather than by raising the global, which would shift every other
     surface by 2.5rem sight-unseen.
     The real fix stays the one `app.css` already names: the shell should read `WindowInsetsCompat`
     and hand the values to the page, the way it hands over `FM_CONFIG_DIR`. Delete this when it does. */
  @media (pointer: coarse) {
    .panel {
      padding-bottom: calc(1.25rem + max(var(--safe-bottom), 3.25rem));
    }
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
  }
  h2 {
    margin: 0;
    font-size: 1.1rem;
  }
  h3 {
    margin: 1.25rem 0 0.25rem;
    font-size: 1rem;
  }
  .close,
  article button {
    /* Own sizing, not `.icon-btn`: those are hidden in the narrow layouts, which is how a control that
       must stay reachable on a phone silently vanishes. */
    min-height: 2.75rem;
    min-width: 2.75rem;
    padding: 0 0.9rem;
    border-radius: 0.5rem;
    border: 1px solid var(--border);
    background: var(--surface);
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  article button:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .total {
    margin: 0.25rem 0;
  }
  .kinds {
    margin: 0.5rem 0 0.75rem;
    padding-left: 1.1rem;
  }
  .kinds li {
    margin: 0.2rem 0;
    font-size: 0.9rem;
  }
  .notes {
    margin: 0 0 0.75rem;
    padding: 0;
    list-style: none;
    max-height: 40vh;
    overflow: auto;
    /* Wide rows scroll inside their own container rather than pushing the panel sideways. */
    overflow-x: auto;
  }
  .notes li {
    display: flex;
    gap: 0.5rem;
    align-items: baseline;
    padding: 0.2rem 0;
    border-bottom: 1px solid var(--border);
    font-size: 0.85rem;
  }
  .role,
  .copies {
    flex: 0 0 auto;
    font-size: 0.75rem;
    color: var(--text-muted);
  }
  .copies {
    /* Loud, because a repeated body is the thing worth noticing in a long list. */
    color: var(--text);
    font-weight: 600;
  }
  .kind {
    flex: 0 0 auto;
    padding: 0 0.35rem;
    border-radius: 0.25rem;
    background: var(--surface);
    color: var(--text-muted);
    font-size: 0.75rem;
  }
  .title {
    flex: 1 1 auto;
    overflow-wrap: anywhere;
  }
  .meta {
    flex: 0 0 auto;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }
  .more,
  .after {
    margin: 0.25rem 0 0;
    font-size: 0.85rem;
    color: var(--text-muted);
  }
</style>
