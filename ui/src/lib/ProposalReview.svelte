<script lang="ts">
  // Show a proposal's change for review: the files it touches and its unified diff against `main`,
  // fetched via `proposalDiff`. A proposal whose branch is gone (merged or deleted) is reported
  // plainly — "nothing to show" — because the proposal note outlives its branch.
  import {
    proposalDiff,
    proposalContent,
    acceptProposal,
    rejectProposal,
    createProposal,
    proposalShown,
  } from './ipc';
  import type { ProposalDiff, ProposalContent } from './types';

  // `onaccepted`/`onrejected` let the parent (the Collaboration list) refresh once a proposal is
  // merged or discarded.
  let { id, onaccepted, onrejected }: { id: string; onaccepted?: () => void; onrejected?: () => void } =
    $props();

  let diff = $state<ProposalDiff | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(true);

  // The PROPOSED note itself — shown and EDITABLE, so a reviewer can change it before accepting. Saving
  // rebuilds the branch from the current `main` (through create_proposal), which also RESOLVES a stale
  // conflict — the fix for "it doesn't merge cleanly and I have no way to change it".
  //
  // This is a DELIBERATELY MINIMAL quick-edit (a plain textarea over the proposed markdown), NOT the
  // app's primary editor. The powerful in-app editor lives in NotePanel; for a heavier revision, refine
  // the proposal by replying in the note's discussion (@name /research|/propose) or accept then edit the
  // note. Kept here so a small tweak/conflict-resolve needs no round-trip. Raw textarea + text-node diff
  // only (no raw-HTML sink), so an untrusted proposed body can never inject.
  let proposed = $state<ProposalContent | null>(null);
  let draft = $state('');
  let saving = $state(false);
  const dirty = $derived(proposed !== null && draft !== proposed.body);

  // The reviewer's own sentence about what they changed. It is the single most valuable thing this
  // screen can collect — git records *what* changed and can never record *why*, and an edit pair
  // stored without its reason measured BELOW the untouched model on hard prompts.
  //
  // OPTIONAL, and always-visible rather than a confirmation step. Both are deliberate: a required
  // prompt produces satisficing instead of reasons, and a field you must dismiss would tax every
  // one-word typo fix. Left blank it costs nothing and the record is simply marked lower-tier.
  let why = $state('');
  // A short controlled vocabulary entered as inline @tags rather than a dropdown: a sentence stays a
  // sentence, one tap covers the common cases, and rejections still aggregate.
  const WHY_TAGS = ['@wrong', '@incomplete', '@style'];
  function addTag(t: string) {
    why = why.trim() ? `${why.trim()} ${t}` : t;
  }
  /** Undefined rather than '' so the backend sees a genuine skip, not an empty reason. */
  const reason = () => why.trim() || undefined;

  // What KIND of correction this was. The one axis the corpus cannot be split on later, and the one
  // that decides what it is good for: style and formatting saturate after roughly a thousand
  // examples, while factual and reasoning corrections keep improving with more data. Unlabelled,
  // the two are indistinguishable and the whole corpus is only as useful as its weakest part.
  //
  // A closed set of three, single-select, and optional — a free-text axis is one nobody can group
  // by, and grouping by it is the entire point.
  const KINDS = ['style', 'factual', 'reasoning'];
  let kind = $state<string | null>(null);
  const pickKind = (k: string) => (kind = kind === k ? null : k);

  /** Push `draft` onto the proposal's branch. Returns false — with the reason already in
   *  `acceptMsg` — so a caller that must NOT proceed on failure (Accept) can stop. */
  async function persistDraft(): Promise<boolean> {
    if (!proposed) return false;
    try {
      await createProposal(proposed.host, draft, reason(), kind ?? undefined); // revises this PR's branch from the current note
      await refetch();
      return true;
    } catch (e) {
      acceptMsg = `Couldn't save: ${e instanceof Error ? e.message : String(e)}`;
      return false;
    }
  }

  async function save() {
    if (!proposed || !dirty || saving) return;
    saving = true;
    acceptMsg = null;
    if (await persistDraft()) {
      acceptMsg = 'Saved — the proposal now holds your edits, rebased on the latest note.';
      why = '';
      kind = null;
    }
    saving = false;
  }

  // The accept (merge) round-trip — the GUI's stand-in for `git merge`, since a UI-only user has none.
  let accepting = $state(false);
  let acceptMsg = $state<string | null>(null);

  // Reject = the safe inverse of accept: discard the proposal's branch (main is never touched).
  // Two-click so a stray tap can't throw away a proposal; the second click within confirms.
  let rejecting = $state(false);
  let confirmingReject = $state(false);

  async function reject() {
    if (!confirmingReject) {
      confirmingReject = true;
      return;
    }
    confirmingReject = false;
    rejecting = true;
    acceptMsg = null;
    try {
      await rejectProposal(id, reason());
      acceptMsg = 'Rejected — the change was dropped and kept as a declined record. main is untouched.';
      why = '';
      await refetch();
      onrejected?.();
    } catch (e) {
      acceptMsg = `Couldn't reject: ${e instanceof Error ? e.message : String(e)}`;
    } finally {
      rejecting = false;
    }
  }

  async function accept() {
    accepting = true;
    acceptMsg = null;
    try {
      // Accepting with unsaved edits used to merge the ORIGINAL and silently drop what the
      // reviewer had typed — `acceptProposal` sends only the id, never the draft. Save first:
      // the edited text is what "accept" means when someone has changed it.
      if (dirty && !(await persistDraft())) return; // persistDraft set the message; `finally` clears the flag
      const { outcome } = await acceptProposal(id);
      if (outcome === 'conflicted') {
        acceptMsg = "This proposal doesn't merge cleanly onto the current note — main was left unchanged. Ask the author to redo it against the latest, or resolve it after a sync.";
      } else {
        // 'merged' or 'already_gone' → it is on main now. Re-fetch: the branch is gone, so the diff
        // flips to the "merged" message, and let the parent refresh its list.
        acceptMsg = outcome === 'already_gone' ? 'Already merged.' : 'Merged into main.';
        await refetch();
        onaccepted?.();
      }
    } catch (e) {
      acceptMsg = `Couldn't accept: ${e instanceof Error ? e.message : String(e)}`;
    } finally {
      accepting = false;
    }
  }

  async function refetch() {
    try {
      diff = await proposalDiff(id);
      proposed = diff.exists ? await proposalContent(id) : null;
      if (proposed) draft = proposed.body;
    } catch (e) {
      error = String(e);
    }
  }

  // Re-fetch whenever the id changes; ignore a stale response if it swaps mid-flight.
  $effect(() => {
    const which = id;
    let cancelled = false;
    loading = true;
    error = null;
    diff = null;
    proposed = null;
    (async () => {
      try {
        const d = await proposalDiff(which);
        if (cancelled) return;
        diff = d;
        const c = d.exists ? await proposalContent(which) : null;
        if (cancelled) return;
        proposed = c;
        if (c) draft = c.body;
        loading = false;
        // It is on screen now. Best-effort and idempotent: this is a note ABOUT the review, and a
        // failure to take it must never break the review itself.
        if (c) void proposalShown(which).catch(() => {});
      } catch (e) {
        if (!cancelled) {
          error = String(e);
          loading = false;
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  });

  const lines = $derived((diff?.patch ?? '').split('\n'));

  // A unified-diff line's role, for colouring. `+++`/`---` are file headers, not adds/deletes.
  function role(line: string): string {
    if (line.startsWith('@@')) return 'hunk';
    if (line.startsWith('+++') || line.startsWith('---') || line.startsWith('diff ')) return 'meta';
    if (line.startsWith('+')) return 'add';
    if (line.startsWith('-')) return 'del';
    return 'ctx';
  }
</script>

<div class="review">
  {#if loading}
    <p class="muted">Loading the proposed change…</p>
  {:else if error}
    <p class="error">Couldn't load the diff: {error}</p>
  {:else if diff && diff.declined}
    <p class="muted">This proposal was declined; the change was dropped, main untouched, and it is kept as a record.</p>
  {:else if diff && !diff.exists}
    <p class="muted">This proposal was merged into the note; the discussion lives on.</p>
  {:else if diff}
    {#if proposed}
      <p class="pr-edit-label">Proposed note{proposed.title ? ` for “${proposed.title}”` : ''} — edit it here before accepting:</p>
      <textarea class="pr-edit" bind:value={draft} rows="12" aria-label="proposed note body"></textarea>
      <div class="pr-edit-actions">
        <button class="pr-save" onclick={save} disabled={!dirty || saving || accepting}>
          {saving ? 'Saving…' : 'Save changes'}
        </button>
        {#if dirty}<span class="pr-dirty">unsaved edits</span>{/if}
      </div>
    {/if}
    <details class="pr-diff-details">
      <summary>View the diff against the current note{#if diff.files.length} · {diff.files.join(', ')}{/if}</summary>
      <pre class="diff">{#each lines as line}<code class="line {role(line)}">{line}</code>{/each}</pre>
    </details>
    <div class="pr-why">
      <label class="pr-why-label" for={`pr-why-${id}`}>
        {confirmingReject ? 'Why are you turning this down?' : 'What did you change?'}
        <span class="pr-optional">optional</span>
      </label>
      <input
        id={`pr-why-${id}`}
        class="pr-why-input"
        bind:value={why}
        placeholder="in your own words — or leave it blank"
      />
      <div class="pr-why-tags">
        {#each WHY_TAGS as t}
          <button type="button" class="pr-why-tag" onclick={() => addTag(t)}>{t}</button>
        {/each}
      </div>
      <div class="pr-why-tags pr-kind">
        <span class="pr-kind-label">What kind?</span>
        {#each KINDS as k}
          <button
            type="button"
            class="pr-why-tag"
            class:picked={kind === k}
            aria-pressed={kind === k}
            onclick={() => pickKind(k)}>{k}</button
          >
        {/each}
      </div>
    </div>
    <div class="review-actions">
      <button
        class="reject"
        class:confirming={confirmingReject}
        onclick={reject}
        disabled={rejecting || accepting}
      >
        {rejecting ? 'Rejecting…' : confirmingReject ? 'Confirm reject' : 'Reject'}
      </button>
      <button class="accept" onclick={accept} disabled={accepting || rejecting}>
        {accepting ? 'Merging…' : 'Accept & merge'}
      </button>
    </div>
  {/if}

  {#if acceptMsg}
    <p class="accept-msg" class:muted={!acceptMsg.startsWith("Couldn't") && !acceptMsg.startsWith('This')}>
      {acceptMsg}
    </p>
  {/if}
</div>

<style>
  .review {
    padding: 0.25rem 0 0.5rem;
  }
  .pr-edit-label {
    margin: 0 0 0.3rem;
    font-size: 0.85em;
    color: var(--muted);
  }
  .pr-edit {
    width: 100%;
    box-sizing: border-box;
    font: inherit;
    font-size: 0.88em;
    line-height: 1.45;
    padding: 0.5rem 0.6rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
    resize: vertical;
  }
  .pr-edit-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin: 0.35rem 0;
  }
  .pr-save {
    min-height: 2rem;
    padding: 0 12px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface-hover);
    color: var(--text);
    font: inherit;
    font-size: 0.85rem;
    cursor: pointer;
  }
  .pr-save:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .pr-dirty {
    font-size: 0.8em;
    color: var(--warning, #b7791f);
  }
  .pr-diff-details {
    margin: 0.4rem 0;
  }
  .pr-diff-details summary {
    font-size: 0.82em;
    color: var(--muted);
    cursor: pointer;
  }
  @media (pointer: coarse) {
    .pr-save {
      min-height: 2.75rem;
    }
  }
  .pr-why {
    margin: 0.5rem 0 0.15rem;
  }
  .pr-why-label {
    display: block;
    font-size: 0.85em;
    color: var(--muted);
    margin-bottom: 0.25rem;
  }
  .pr-optional {
    font-size: 0.9em;
    opacity: 0.7;
  }
  .pr-why-input {
    width: 100%;
    box-sizing: border-box;
    font: inherit;
    font-size: 0.88em;
    padding: 0.4rem 0.55rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
  }
  .pr-why-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
    margin-top: 0.3rem;
  }
  .pr-kind {
    align-items: center;
  }
  .pr-kind-label {
    font-size: 0.78em;
    color: var(--muted);
  }
  .pr-why-tag.picked {
    border-color: var(--accent, #6639ba);
    color: var(--accent, #6639ba);
  }
  .pr-why-tag {
    border: 1px solid var(--border);
    border-radius: 999px;
    background: transparent;
    color: var(--muted);
    font: inherit;
    font-size: 0.78em;
    padding: 1px 8px;
    cursor: pointer;
  }
  @media (pointer: coarse) {
    .pr-why-input {
      min-height: 2.75rem;
    }
    .pr-why-tag {
      min-height: 2.75rem;
      padding: 0 12px;
    }
  }
  .review-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-top: 0.5rem;
  }
  button.reject {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text);
    padding: 4px 12px;
    border-radius: var(--radius-2, 6px);
    font-size: 0.85rem;
    cursor: pointer;
  }
  button.reject.confirming {
    border-color: var(--danger, #c0392b);
    color: var(--danger, #c0392b);
  }
  button.reject:disabled {
    opacity: 0.6;
    cursor: default;
  }
  button.accept {
    background: var(--accent, #6639ba);
    border: 1px solid var(--accent, #6639ba);
    color: #fff;
    padding: 4px 12px;
    border-radius: var(--radius-2, 6px);
    font-size: 0.85rem;
    cursor: pointer;
  }
  button.accept:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .accept-msg {
    margin: 0.4rem 0 0;
    font-size: 0.9em;
    color: var(--danger, #c0392b);
  }
  .accept-msg.muted {
    color: var(--success, #1a7f37);
  }
  .muted {
    color: var(--muted);
    font-size: 0.9em;
  }
  .error {
    color: var(--danger, #c0392b);
    font-size: 0.9em;
  }
  .files {
    color: var(--muted);
    font-size: 0.85em;
    margin: 0 0 0.35rem;
    word-break: break-all;
  }
  .diff {
    margin: 0;
    padding: 0.5rem 0.6rem;
    overflow-x: auto;
    background: var(--surface-2, rgba(127, 127, 127, 0.08));
    border-radius: 6px;
    font-size: 0.82em;
    line-height: 1.4;
  }
  .line {
    display: block;
    white-space: pre;
  }
  .line.add {
    color: #1a7f37;
    background: rgba(46, 160, 67, 0.12);
  }
  .line.del {
    color: #cf222e;
    background: rgba(207, 34, 46, 0.12);
  }
  .line.hunk {
    color: var(--accent, #6639ba);
  }
  .line.meta {
    color: var(--muted);
  }
  @media (prefers-color-scheme: dark) {
    .line.add {
      color: #3fb950;
    }
    .line.del {
      color: #f85149;
    }
  }
</style>
