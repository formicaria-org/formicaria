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
  import { formatStamp, toStamp } from './stamp';
  import { STATE_LABEL } from './proposalState';
  import { formatDay } from './appTime';
  import LiveEditor from './editor/LiveEditor.svelte';

  // `onaccepted`/`onrejected` let the parent (the Collaboration list) refresh once a proposal is
  // merged or discarded.
  let {
    id,
    onaccepted,
    onrejected,
  }: { id: string; onaccepted?: () => void; onrejected?: () => void } = $props();

  let diff = $state<ProposalDiff | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(true);

  // The PROPOSED note itself — shown and EDITABLE, so a reviewer can change it before accepting. Saving
  // rebuilds the branch from the current `main` (through create_proposal), which also RESOLVES a stale
  // conflict — the fix for "it doesn't merge cleanly and I have no way to change it".
  //
  // **The note's own editor** (`LiveEditor`), since 2026-10-15: the proposed note is drawn finished,
  // and a line turns into its text where it is clicked — exactly as a note does (the owner: a
  // proposal "should behave exactly as the current notes"). It was a plain textarea until then. The
  // editor already draws collaborators' untrusted notes, under the audited no-raw-HTML-sink rule
  // (`ci/checks.sh`), so a proposed body is no new exposure. The diff below stays text nodes only.
  let proposed = $state<ProposalContent | null>(null);
  let draft = $state('');
  let saving = $state(false);

  // The proposal's DETAILS — when and where — shown as fields beside what the note has now, and
  // editable like the text: a meeting found at the wrong time is corrected here, not rejected and
  // re-entered. Split day/time exactly as NotePanel's Details do, because a time is optional.
  let dStart = $state('');
  let dStartTime = $state('');
  let dDue = $state('');
  let dDueTime = $state('');
  let dLocation = $state('');
  let dTitle = $state('');
  const split = (s?: string | null) => {
    const [day = '', time = ''] = (s ?? '').split('T');
    return [day, time] as const;
  };
  function loadDetails(c: ProposalContent) {
    [dStart, dStartTime] = split(c.start);
    [dDue, dDueTime] = split(c.due);
    dLocation = c.location ?? '';
    dTitle = c.title;
  }
  /** The proposal renames the note — a conversation's "Person — who they are" (2026-10-15). */
  const renames = $derived(
    proposed !== null &&
      !proposed.isNew &&
      !!proposed.wasTitle &&
      proposed.title !== proposed.wasTitle,
  );
  /** When and where: a proposal that touches neither shows no date fields. */
  const hasWhen = $derived(
    proposed !== null && [proposed.start, proposed.due, proposed.location].some(Boolean),
  );
  // Only the fields the section shows: a rename alone must not send an empty date, which would clear
  // the meeting the note already has.
  const details = $derived({
    ...(hasWhen
      ? {
          start: toStamp(dStart, dStartTime),
          due: toStamp(dDue, dDueTime),
          location: dLocation.trim(),
        }
      : {}),
    ...(renames ? { title: dTitle.trim() } : {}),
  });
  /** Only a proposal that touches the details gets the section; a text-only one stays as it was. */
  const hasDetails = $derived(renames || hasWhen);
  const detailsDirty = $derived(
    proposed !== null &&
      ((hasWhen &&
        (details.start !== (proposed.start ?? '') ||
          details.due !== (proposed.due ?? '') ||
          details.location !== (proposed.location ?? ''))) ||
        (renames && dTitle.trim() !== proposed.title)),
  );
  const dirty = $derived(proposed !== null && (draft !== proposed.body || detailsDirty));
  /** "now: …" under a field whose value would change; nothing when it would not. */
  function now(
    was: string | null | undefined,
    next: string | undefined,
    stamp: boolean,
  ): string | null {
    if (!proposed || proposed.isNew || (was ?? '') === (next ?? '')) return null;
    return was ? `now: ${stamp ? formatStamp(was) : was}` : 'now: not set';
  }

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
      // Revises this PR's branch from the current note; corrected details go over the stored ones.
      await createProposal(
        proposed.host,
        draft,
        reason(),
        kind ?? undefined,
        detailsDirty ? details : undefined,
      );
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
      acceptMsg =
        'Rejected — the change was dropped and kept as a declined record. main is untouched.';
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
        acceptMsg =
          "This proposal doesn't merge cleanly onto the current note — main was left unchanged. Ask the author to redo it against the latest, or resolve it after a sync.";
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
      if (proposed) {
        draft = proposed.body;
        loadDetails(proposed);
      }
    } catch (e) {
      error = String(e);
    }
  }

  // Re-fetch whenever the id changes; ignore a stale response if it swaps mid-flight.
  //
  // **Through a `$derived`, which passes on only a changed value.** The parent hands `id={p.id}`
  // from a list item, and every refresh replaces the item with an equal copy; read directly, that
  // re-ran this effect, blanked the review to "Loading…" and recorded its showing again — the
  // Collaboration view flickering every 15 s (2026-10-15).
  const currentId = $derived(id);
  $effect(() => {
    const which = currentId;
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
        if (c) {
          draft = c.body;
          loadDetails(c);
        }
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
  {:else if diff && (diff.declined || !diff.exists)}
    <!-- **Closed: how it ended, and what it did** (2026-10-24), read-only, like a closed pull
         request. The change comes from git's record (the accept commit, or the text turned down). -->
    <div class="pr-closed">
      <p class="pr-closed-line">
        <span class="pr-closed-state" data-state={diff.state}>{STATE_LABEL[diff.state]}</span>
        {#if diff.closed_at}<span class="muted"> · {formatDay(diff.closed_at)}</span>{/if}
        <span class="muted">
          — {diff.state === 'merged'
            ? 'accepted into the note.'
            : diff.state === 'declined'
              ? 'turned down; the note was not changed.'
              : 'closed without being accepted or turned down here.'}</span
        >
      </p>
      {#if diff.why}
        <p class="pr-closed-why">Why: “{diff.why}”</p>
      {/if}
      {#if diff.shown_from_history && diff.patch}
        <p class="muted">
          {diff.state === 'merged' ? 'What it changed' : 'What it proposed'}{#if diff.files.length}
            · {diff.files.join(', ')}{/if}
        </p>
        <pre class="diff">{#each lines as line}<code class="line {role(line)}">{line}</code
            >{/each}</pre>
      {:else}
        <p class="muted">
          {diff.state === 'declined'
            ? 'The text that was turned down is not kept on this device; the reason above is its record.'
            : 'There is no record of its change on this device.'}
        </p>
      {/if}
    </div>
  {:else if diff}
    {#if proposed}
      {#if hasDetails}
        <!-- A proposal that dates a note shows the details it sets as fields, beside what the note
             has now, and lets them be corrected: accepting a meeting time should never depend on
             opening the diff, nor a wrong one on rejecting the whole proposal. -->
        <fieldset class="pr-details" data-testid="pr-details">
          <legend
            >{proposed.isNew ? 'New meeting' : 'Details'} — check or correct before accepting</legend
          >
          {#if renames}
            <label class="pr-field">
              <span>Title</span>
              <input aria-label="proposed title" bind:value={dTitle} spellcheck="false" />
              {#if dTitle.trim() !== proposed.wasTitle}
                <small class="pr-was">now: {proposed.wasTitle}</small>
              {/if}
            </label>
          {/if}
          {#if hasWhen}
            <label class="pr-field">
              <span>Start</span>
              <span class="pr-when">
                <input aria-label="proposed start" type="date" bind:value={dStart} />
                <input
                  aria-label="proposed start time"
                  type="time"
                  bind:value={dStartTime}
                  disabled={!dStart}
                />
              </span>
              {#if now(proposed.wasStart, details.start, true)}
                <small class="pr-was">{now(proposed.wasStart, details.start, true)}</small>
              {/if}
            </label>
            <label class="pr-field">
              <span>Due</span>
              <span class="pr-when">
                <input aria-label="proposed due" type="date" bind:value={dDue} />
                <input
                  aria-label="proposed due time"
                  type="time"
                  bind:value={dDueTime}
                  disabled={!dDue}
                />
              </span>
              {#if now(proposed.wasDue, details.due, true)}
                <small class="pr-was">{now(proposed.wasDue, details.due, true)}</small>
              {/if}
            </label>
            <label class="pr-field">
              <span>Place</span>
              <input
                aria-label="proposed place"
                bind:value={dLocation}
                placeholder="a room, or a meeting link"
                spellcheck="false"
              />
              {#if now(proposed.wasLocation, details.location, false)}
                <small class="pr-was">{now(proposed.wasLocation, details.location, false)}</small>
              {/if}
            </label>
          {/if}
        </fieldset>
      {/if}
      <p class="pr-edit-label">
        Proposed note{proposed.title ? ` for “${proposed.title}”` : ''} — edit it here before accepting:
      </p>
      <!-- The note's own editor: drawn finished, a line turning into its text where you click —
           a proposal reads like the note it would become (the owner, 2026-10-15). -->
      <div class="pr-edit">
        <LiveEditor
          bind:value={draft}
          label="proposed note body"
          undoLabel="typing in the proposed note"
        />
      </div>
      <div class="pr-edit-actions">
        <button class="pr-save" onclick={save} disabled={!dirty || saving || accepting}>
          {saving ? 'Saving…' : 'Save changes'}
        </button>
        {#if dirty}<span class="pr-dirty">unsaved edits</span>{/if}
      </div>
    {/if}
    <details class="pr-diff-details">
      <summary
        >View the diff against the current note{#if diff.files.length}
          · {diff.files.join(', ')}{/if}</summary
      >
      <pre class="diff">{#each lines as line}<code class="line {role(line)}">{line}</code
          >{/each}</pre>
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
    <p
      class="accept-msg"
      class:muted={!acceptMsg.startsWith("Couldn't") && !acceptMsg.startsWith('This')}
    >
      {acceptMsg}
    </p>
  {/if}
</div>

<style>
  .review {
    padding: 0.25rem 0 0.5rem;
  }
  .pr-details {
    margin: 0 0 var(--space-2);
    padding: 0.5rem 0.6rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    display: grid;
    gap: 0.45rem;
    min-width: 0;
  }
  .pr-details legend {
    font-size: 0.85em;
    color: var(--muted);
    padding: 0 0.25rem;
  }
  .pr-field {
    display: grid;
    gap: 0.15rem;
    min-width: 0;
    font-size: 0.88em;
  }
  .pr-when {
    display: flex;
    gap: var(--space-1);
    min-width: 0;
  }
  .pr-when input[type='date'] {
    flex: 1;
    min-width: 0;
  }
  .pr-field input {
    box-sizing: border-box;
    max-width: 100%;
    font: inherit;
    padding: 0.3rem 0.45rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
  }
  .pr-was {
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .pr-edit-label {
    margin: 0 0 0.3rem;
    font-size: 0.85em;
    color: var(--muted);
  }
  .pr-edit {
    width: 100%;
    box-sizing: border-box;
    min-height: 8rem;
    max-height: 60dvh;
    overflow: auto;
    padding: 0.5rem 0.6rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
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
    .pr-field input {
      min-height: 2.75rem;
    }
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
  .pr-closed-line {
    margin: 0 0 var(--space-1);
  }
  .pr-closed-state {
    font-weight: 600;
  }
  .pr-closed-state[data-state='merged'] {
    color: var(--accent);
  }
  .pr-closed-why {
    margin: 0 0 var(--space-1);
  }
</style>
