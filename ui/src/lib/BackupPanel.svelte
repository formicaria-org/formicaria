<script lang="ts">
  // Backing up in two tiers, and saying which one you actually got.
  //
  // Light (the default): commit + push the notes. Text only — small, plain, and
  // authenticated by whatever ssh-agent or credential helper the user already
  // has, so this app stores no secret.
  // Heavy (tick to include): restic over the whole vault, blobs and all.
  //
  // The panel's whole job is to never overstate. It says what each tier will and
  // will not carry *before* you act, and afterwards reports each tier's real
  // outcome — including whether the data left this machine at all, which a local
  // path for a remote or a restic repo quietly does not.
  import { onMount } from 'svelte';
  import { backup, backupStatus, commit, pull, push, setGitRemote } from './ipc';
  import { reachOf, shortDest } from './destination';
  import type { BackupStatus } from './types';

  let { onclose }: { onclose: () => void } = $props();

  type Step = { text: string; ok: boolean };

  let status = $state<BackupStatus | null>(null);
  let remoteDraft = $state('');
  let nameDraft = $state('');
  let emailDraft = $state('');
  let heavy = $state(false);
  let busy = $state(false);
  let steps = $state<Step[]>([]);
  let verdict = $state<string | null>(null);
  let error = $state<string | null>(null);

  const notesReach = $derived(reachOf(status?.remote));
  const mediaReach = $derived(reachOf(status?.restic_repo));
  const canPush = $derived(!!status?.remote && !busy);
  // Only the people git has never met get asked. Loaded status first, so the
  // question doesn't flash up before we know the answer.
  const needsIdentity = $derived(!!status && status.identity === null);
  const canSaveRemote = $derived(
    !busy &&
      !!remoteDraft.trim() &&
      (!needsIdentity || (!!nameDraft.trim() && !!emailDraft.trim())),
  );

  const msg = (e: unknown) => (e instanceof Error ? e.message : String(e));
  const leaves = (r: string) => (r === 'remote' ? 'leaves this machine' : 'stays on this machine');
  const left = (r: string) => (r === 'remote' ? 'off this machine' : 'still on this machine');

  onMount(load);

  async function load() {
    try {
      status = await backupStatus();
      remoteDraft = status.remote ?? '';
    } catch (e) {
      error = msg(e);
    }
  }

  async function saveRemote() {
    const url = remoteDraft.trim();
    if (!url || busy) return;
    error = null;
    const name = nameDraft.trim();
    const email = emailDraft.trim();
    try {
      await setGitRemote(url, needsIdentity ? { name, email } : undefined);
      await load();
    } catch (e) {
      error = msg(e);
    }
  }

  // Bring their work home. Separate from "Back up" on purpose: pushing and pulling are
  // different intentions, and a button that quietly did both would be a button nobody
  // could predict. Commit first for the same reason `run()` does — the 5s auto-commit
  // is best-effort, and git will not merge over uncommitted edits.
  async function bringDown() {
    if (busy) return;
    busy = true;
    steps = [];
    verdict = null;
    error = null;
    try {
      await commit(`auto: ${new Date().toISOString()}`).catch(() => false);
      const r = await pull();
      if (r.conflicts.length) {
        steps.push({
          text: `Merged, but ${r.conflicts.length} note${r.conflicts.length === 1 ? '' : 's'} need you: ${r.conflicts.join(', ')}. Open each one — the two versions are marked in the text.`,
          ok: false,
        });
      } else if (r.merged) {
        steps.push({ text: `Pulled ${r.merged} commit${r.merged === 1 ? '' : 's'} — merged cleanly.`, ok: true });
      } else {
        steps.push({ text: 'Already up to date.', ok: true });
      }
    } catch (e) {
      steps.push({ text: `Could not pull: ${msg(e)}`, ok: false });
    }
    await load();
    busy = false;
  }

  async function run() {
    if (!status || busy) return;
    busy = true;
    steps = [];
    verdict = null;
    error = null;
    const now = new Date().toISOString();
    let notesOff = false;
    let mediaOff = false;

    // Flush whatever the 5s auto-commit debounce has not written yet: it is
    // best-effort and swallows its errors, so never assume it has run.
    try {
      const made = await commit(`auto: ${now}`);
      steps.push({ text: made ? 'Committed your latest changes.' : 'Nothing new to commit.', ok: true });
    } catch (e) {
      steps.push({ text: `Could not commit: ${msg(e)}`, ok: false });
    }

    // The tiers are independent — a failed push must not cancel a full backup,
    // and each reports its own fate.
    try {
      const squashed = await push(`backup: ${now}`);
      if (squashed > 1) {
        steps.push({ text: `Squashed ${squashed} commits into one.`, ok: true });
      }
      steps.push({
        text: `Notes pushed to ${shortDest(status.remote ?? '')} — ${left(notesReach)}.`,
        ok: true,
      });
      notesOff = notesReach === 'remote';
    } catch (e) {
      steps.push({ text: `Notes NOT pushed: ${msg(e)}`, ok: false });
    }

    if (heavy) {
      try {
        await backup();
        steps.push({
          text: `Full backup, media included → ${shortDest(status.restic_repo ?? '')} — ${left(mediaReach)}.`,
          ok: true,
        });
        mediaOff = mediaReach === 'remote';
      } catch (e) {
        steps.push({ text: `Full backup failed: ${msg(e)}`, ok: false });
      }
    }

    verdict =
      `Your notes are ${notesOff ? 'off' : 'still on'} this machine. ` +
      (heavy
        ? `Your media is ${mediaOff ? 'off' : 'still on'} this machine.`
        : 'Your media was not included.');
    await load();
    busy = false;
  }
</script>

<svelte:window
  onkeydown={(e) => {
    if (e.key === 'Escape' && !busy) onclose();
  }}
/>

<div class="backup-overlay">
  <button class="backup-backdrop" aria-label="close backup" onclick={() => !busy && onclose()}
  ></button>
  <div class="panel" role="dialog" aria-label="back up">
    <h2>Back up</h2>

    <label class="remote">
      <span>Your notes' git remote</span>
      <div class="row">
        <!-- svelte-ignore a11y_autofocus -->
        <input
          bind:value={remoteDraft}
          onkeydown={(e) => e.key === 'Enter' && canSaveRemote && saveRemote()}
          placeholder="git@github.com:you/notes.git"
          spellcheck="false"
          disabled={busy}
        />
        <button onclick={saveRemote} disabled={!canSaveRemote}>Save</button>
      </div>
    </label>

    {#if needsIdentity}
      <!-- Asked once, at the only moment it matters: pushing is when your notes
           start carrying your name to someone else, and git history is permanent.
           Anyone who has ever configured git never sees this. -->
      <div class="identity">
        <p class="why">
          Every commit you push is signed with a name. Yours isn't set yet — without
          it, your collaborators can't tell who changed what.
        </p>
        <div class="row">
          <input
            bind:value={nameDraft}
            onkeydown={(e) => e.key === 'Enter' && canSaveRemote && saveRemote()}
            placeholder="Your name"
            spellcheck="false"
            disabled={busy}
          />
          <input
            bind:value={emailDraft}
            onkeydown={(e) => e.key === 'Enter' && canSaveRemote && saveRemote()}
            placeholder="you@example.org"
            spellcheck="false"
            disabled={busy}
          />
        </div>
      </div>
    {/if}

    {#if steps.length === 0}
      <ul class="promise">
        <li>
          {#if notesReach === 'unset'}
            <strong>No remote set</strong> — your notes cannot leave this machine yet.
          {:else}
            Notes → <strong>{shortDest(status?.remote ?? '')}</strong> — {leaves(notesReach)}.
            {#if status?.unpushed}
              <span class="muted">{status.unpushed} commit{status.unpushed === 1 ? '' : 's'} not pushed.</span>
            {/if}
            {#if status?.identity}
              <span class="muted">Signed as {status.identity.name} &lt;{status.identity.email}&gt;.</span>
            {/if}
          {/if}
        </li>
        <li>
          {#if !heavy}
            Media in <code>blobs/</code> (images, PDFs, video) is <strong>not included</strong>.
          {:else}
            Media → <strong>{shortDest(status?.restic_repo ?? '')}</strong> — {leaves(mediaReach)}.
          {/if}
        </li>
      </ul>
    {:else}
      <ul class="steps">
        {#each steps as s, i (i)}
          <li class:bad={!s.ok}>{s.text}</li>
        {/each}
      </ul>
      {#if verdict}<p class="verdict">{verdict}</p>{/if}
    {/if}

    <label class="heavy" class:disabled={!status?.restic_ready}>
      <input type="checkbox" bind:checked={heavy} disabled={busy || !status?.restic_ready} />
      <span>
        Include media — full restic backup
        {#if status && !status.restic_ready}
          <span class="muted">
            (unavailable: set <code>FM_RESTIC_REPO</code> and <code>RESTIC_PASSWORD</code>, then
            restart)
          </span>
        {/if}
      </span>
    </label>

    {#if status?.conflicts.length}
      <!-- The one state a user must be told about by name: these notes have both
           versions in them and are waiting for a person. The merge driver keeps the
           markers in the body, so each one still opens in the editor. -->
      <p class="error">
        {status.conflicts.length} note{status.conflicts.length === 1 ? '' : 's'} still need
        you: {status.conflicts.join(', ')}. Open each and keep the text you want.
      </p>
    {:else if status?.remote_moved}
      <p class="moved">Someone has pushed work you don't have yet.</p>
    {/if}

    {#if error}<p class="error">{error}</p>{/if}

    <div class="actions">
      <button onclick={onclose} disabled={busy}>Close</button>
      <button onclick={bringDown} disabled={!canPush} class:primary={status?.remote_moved}>
        {busy ? 'Working…' : 'Get their changes'}
      </button>
      <button class="primary" onclick={run} disabled={!canPush}>
        {busy ? 'Backing up…' : heavy ? 'Back up notes + media' : 'Back up notes'}
      </button>
    </div>
  </div>
</div>

<style>
  .backup-overlay {
    position: fixed;
    inset: 0;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 12vh;
    z-index: 80;
  }
  .backup-backdrop {
    position: fixed;
    inset: 0;
    border: none;
    background: rgb(0 0 0 / 0.45);
    cursor: default;
  }
  .panel {
    position: relative;
    width: min(34rem, 92vw);
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
  h2 {
    margin: 0;
    font-size: var(--text-md);
    color: var(--text);
  }
  .remote {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .row {
    display: flex;
    gap: var(--space-2);
  }
  .identity {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--accent-subtle);
  }
  .why {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text);
    line-height: 1.5;
  }
  .moved {
    margin: 0;
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    background: var(--accent-subtle);
    color: var(--text);
    font-size: var(--text-sm);
  }
  input[type='text'],
  .row input {
    flex: 1;
    min-width: 0;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
  }
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    font-size: var(--text-sm);
    color: var(--text);
  }
  .promise li,
  .steps li {
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    background: var(--accent-subtle);
    line-height: 1.5;
  }
  .steps li {
    background: var(--ok-bg);
    color: var(--ok-fg);
  }
  .steps li.bad {
    background: var(--danger-bg);
    color: var(--danger-fg);
  }
  .verdict {
    margin: 0;
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--text);
  }
  .muted {
    color: var(--text-muted);
    font-weight: 400;
  }
  .heavy {
    display: flex;
    gap: var(--space-2);
    align-items: flex-start;
    font-size: var(--text-sm);
    color: var(--text);
    line-height: 1.5;
  }
  .heavy.disabled {
    color: var(--text-muted);
  }
  code {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
  }
  .error {
    margin: 0;
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    background: var(--danger-bg);
    color: var(--danger-fg);
    font-size: var(--text-sm);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }
  button {
    padding: var(--space-2) var(--space-4);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    font-size: var(--text-sm);
    cursor: pointer;
  }
  button.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
