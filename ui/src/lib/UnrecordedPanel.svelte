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

  import type { DuplicateFamily, Unrecorded } from './types';
  import { labelFor } from './vaultLabels.svelte';

  interface Props {
    unrecorded: Unrecorded[];
    /// Identical-note families. Shown here because this is where the user already is when a duplicate
    /// count sends them looking, and because recording is the step that must come *first*.
    duplicates?: DuplicateFamily[];
    onrecord: (vault: string) => Promise<void> | void;
    onprune: (vault: string) => Promise<void> | void;
    onclose: () => void;
  }
  let { unrecorded, duplicates = [], onrecord, onprune, onclose }: Props = $props();

  /// Which vaults still have notes outside git. Pruning those is refused by the backend — and the
  /// button says so rather than offering an action that will fail.
  const outstanding = $derived(new Set(unrecorded.filter((u) => u.count > 0).map((u) => u.vault)));
  let pruning = $state<string | null>(null);
  async function prune(vault: string) {
    pruning = vault;
    try {
      await onprune(vault);
    } finally {
      pruning = null;
    }
  }
  /// Families per vault, so each vault's extras are one button.
  const byVault = $derived(
    [...new Set(duplicates.map((f) => f.vault))].map((v) => ({
      vault: v,
      families: duplicates.filter((f) => f.vault === v),
      extras: duplicates.filter((f) => f.vault === v).reduce((n, f) => n + f.extras.length, 0),
    })),
  );

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

      <!-- **Actions first, evidence behind a disclosure.** With fifty rows at three lines each, the
           Record button sat a long scroll below the fold on a phone — the thing you act on should not
           be the hardest thing to reach. The rows are what identified this incident, so they stay one
           tap away rather than being cut. -->
      <button type="button" disabled={busy === u.vault} onclick={() => record(u.vault)}>
        {busy === u.vault ? 'Recording…' : `Record all ${u.count} in history`}
      </button>
      <p class="after">Then back up, to send them to a remote.</p>

      {#if u.notes.length}
        <details>
          <summary>Show {u.notes.length} of {u.count}</summary>
        <ul class="notes">
          {#each u.notes as n (n.path)}
            <li>
              <span class="kind" data-kind={n.kind}>{n.kind}</span>
              <span class="role">{n.role}</span>
              <span class="title">{n.title ?? n.id}</span>
              <!-- **The duplicate count is the diagnosis.** "147 notes" says nothing; "one note
                   written 138 times" names a loop. -->
              {#if n.copies > 1}<span class="copies">×{n.copies}</span>{/if}
              <!-- **Both times, and labelled**, because they answer different questions: a copy or a
                   restore resets every file's mtime at once, which makes an old set look like a fresh
                   burst. `created` is the note's own. -->
              <span class="meta">
                {size(n.bytes)}
                {#if n.created}· made {when(n.created)}{/if}
                {#if n.modified && n.created !== n.modified}· written {when(n.modified)}{/if}
              </span>
            </li>
          {/each}
        </ul>
        </details>
      {/if}
    </article>
  {/each}

  {#if !unrecorded.length}
    <p class="total">Nothing outstanding — git has every note.</p>
  {/if}

  <!-- **Duplicates: the same note written more than once.** Separate from "not in history" because it
       is a different problem with a different fix, and because the *order* between them is not
       optional: removing a copy git does not have is unrecoverable. -->
  {#each byVault as v (v.vault)}
    <article>
      <h3>Duplicate copies in {labelFor(v.vault)}</h3>
      <p class="total">
        {v.extras} extra cop{v.extras === 1 ? 'y' : 'ies'} across {v.families.length}
        note{v.families.length === 1 ? '' : 's'}. The oldest of each is kept.
      </p>
      <ul class="notes">
        {#each v.families as f (f.body)}
          <li>
            <span class="copies">×{f.extras.length + 1}</span>
            <span class="title">{f.preview}</span>
          </li>
        {/each}
      </ul>
      {#if outstanding.has(v.vault)}
        <!-- The refusal, said before it is hit: the backend will decline, and an action that cannot
             work should not look available. -->
        <p class="more">
          Record the outstanding notes above first — until then, removing a copy could not be undone.
        </p>
      {:else}
        <button type="button" disabled={pruning === v.vault} onclick={() => prune(v.vault)}>
          {pruning === v.vault ? 'Removing…' : `Remove ${v.extras} extra cop${v.extras === 1 ? 'y' : 'ies'}`}
        </button>
        <p class="after">They stay in git history, so this can be undone.</p>
      {/if}
    </article>
  {/each}
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
  /* **The navigation bar ate the button**, twice — and the second time was self-inflicted.
     0.5rem (the global floor) clears a gesture pill but not a 3-button navigation bar, so this
     panel carried a local `3.25rem`. Observed on the owner's phone: "Record all 147 in history"
     half under the system bar, the line below it invisible.

     It was deleted on 2026-08-31 on the grounds that the shell now reports the real inset — and
     the shell's first attempt at that never reached the page, so the deletion swapped a working
     52px guess for a 28px one and the owner lost the button again the same day.

     **It stays now, bridge or no bridge.** A floor is only ever wrong by being generous; its
     absence is a control nobody can press. `max()` means the device's real number still wins the
     moment it arrives. */
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
  /* **One scroll surface, not two.** This list used to be `max-height: 40vh; overflow: auto`, so a
     finger landing anywhere on it scrolled the *list* and the panel never moved — which made
     everything below (the duplicates section, the action buttons) unreachable on a phone. Observed
     2026-07-31: the owner scrolled twice and saw the same header both times. The panel scrolls; the
     list is just content. */
  .notes {
    margin: 0 0 0.75rem;
    padding: 0;
    list-style: none;
  }
  /* **The title gets its own line.** With the `kind`, `role` and size/date columns all competing on
     one flex row, a narrow screen squeezed the title to about one character per line — "Me / eti / ng"
     for a note called "Meeting". Seen on the phone; no test could show it and svelte-check cannot
     either. Badges and metadata share the first line, the title owns the second. */
  .notes li {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem 0.5rem;
    align-items: baseline;
    padding: 0.3rem 0;
    border-bottom: 1px solid var(--border);
    font-size: 0.85rem;
  }
  .notes .title {
    flex: 1 1 100%;
  }
  .notes .meta {
    margin-left: auto;
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
  summary {
    /* A comfortable touch target, not a 12px triangle. */
    min-height: 2.5rem;
    display: flex;
    align-items: center;
    cursor: pointer;
    color: var(--text-muted);
    font-size: 0.85rem;
  }
  .more,
  .after {
    margin: 0.25rem 0 0;
    font-size: 0.85rem;
    color: var(--text-muted);
  }
</style>
