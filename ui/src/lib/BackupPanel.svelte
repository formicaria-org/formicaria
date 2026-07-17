<script lang="ts">
  // Backing up in two tiers, and saying which one you actually got.
  //
  // Light (the default): commit + push the notes. Text only — small, plain, and
  // authenticated by whatever ssh-agent or credential helper the user already
  // has, so this app stores no secret.
  // Heavy (tick to include): restic over every vault, blobs and all.
  //
  // The panel's whole job is to never overstate. It says what each tier will and
  // will not carry *before* you act, and afterwards reports each tier's real
  // outcome — including whether the data left this machine at all, which a local
  // path for a remote or a restic repo quietly does not.
  //
  // **It is a list, not a form.** Git is per vault by definition — one vault is one
  // repo, one remote, one collaborator list — so each vault gets its own destination,
  // its own identity, its own unpushed count and its own "someone pushed". Collapsing
  // them into one "Back up" that silently meant the first vault is exactly the
  // overstatement this panel exists to prevent. Restic is the exception, and says so:
  // one repo for the whole set, because a snapshot is disaster recovery, not sharing.
  import { onMount } from 'svelte';
  import { backup, backupStatus, commit, pull, push, setGitRemote } from './ipc';
  import { reachOf, shortDest } from './destination';
  import type { BackupStatus, VaultStatus } from './types';

  let { onclose }: { onclose: () => void } = $props();

  type Step = { text: string; ok: boolean };

  let status = $state<BackupStatus | null>(null);
  // Per vault, keyed by name — a shared draft would put your lab remote in your
  // personal vault the moment you looked away.
  let remoteDrafts = $state<Record<string, string>>({});
  let nameDrafts = $state<Record<string, string>>({});
  let emailDrafts = $state<Record<string, string>>({});
  let heavy = $state(false);
  let busy = $state(false);
  let steps = $state<Step[]>([]);
  let verdict = $state<string | null>(null);
  let error = $state<string | null>(null);

  const vaults = $derived(status?.vaults ?? []);
  const mediaReach = $derived(reachOf(status?.restic_repo));
  // Something to push somewhere. A vault with no remote isn't a failure, it just has
  // nowhere to go yet.
  const canRun = $derived(!busy && vaults.some((v) => !!v.remote));
  // Only the people git has never met get asked, and only about the vault they are
  // sharing: a vault is an audience, so the name on a lab repo need not be the one on
  // your personal notes.
  const needsIdentity = (v: VaultStatus) => v.identity === null;
  const canSaveRemote = (v: VaultStatus) =>
    !busy &&
    !!remoteDrafts[v.name]?.trim() &&
    (!needsIdentity(v) || (!!nameDrafts[v.name]?.trim() && !!emailDrafts[v.name]?.trim()));

  const msg = (e: unknown) => (e instanceof Error ? e.message : String(e));
  const leaves = (r: string) => (r === 'remote' ? 'leaves this machine' : 'stays on this machine');
  const left = (r: string) => (r === 'remote' ? 'off this machine' : 'still on this machine');
  // A single vault has no boundary to talk about, so don't name it at every turn.
  const plural = $derived(vaults.length > 1);
  const of = (v: VaultStatus) => (plural ? ` (${v.name})` : '');

  onMount(load);

  async function load() {
    try {
      status = await backupStatus();
      for (const v of status.vaults) {
        remoteDrafts[v.name] ??= v.remote ?? '';
        nameDrafts[v.name] ??= '';
        emailDrafts[v.name] ??= '';
      }
    } catch (e) {
      error = msg(e);
    }
  }

  async function saveRemote(v: VaultStatus) {
    const url = remoteDrafts[v.name]?.trim();
    if (!url || busy) return;
    error = null;
    try {
      const identity = needsIdentity(v)
        ? { name: nameDrafts[v.name].trim(), email: emailDrafts[v.name].trim() }
        : undefined;
      await setGitRemote(url, identity, v.name);
      await load();
    } catch (e) {
      error = msg(e);
    }
  }

  // Bring their work home. Separate from "Back up" on purpose: pushing and pulling are
  // different intentions, and a button that quietly did both would be a button nobody
  // could predict. Commit first for the same reason `run()` does — the 5s auto-commit
  // is best-effort, and git will not merge over uncommitted edits.
  async function bringDown(v: VaultStatus) {
    if (busy) return;
    busy = true;
    steps = [];
    verdict = null;
    error = null;
    try {
      await commit(`auto: ${new Date().toISOString()}`, v.name).catch(() => false);
      const r = await pull(v.name);
      if (r.conflicts.length) {
        steps.push({
          text: `Merged${of(v)}, but ${r.conflicts.length} note${r.conflicts.length === 1 ? '' : 's'} need you: ${r.conflicts.join(', ')}. Open each one — both versions are marked in the text.`,
          ok: false,
        });
      } else if (r.merged) {
        steps.push({
          text: `Pulled ${r.merged} commit${r.merged === 1 ? '' : 's'}${of(v)} — merged cleanly.`,
          ok: true,
        });
      } else {
        steps.push({ text: `Already up to date${of(v)}.`, ok: true });
      }
    } catch (e) {
      steps.push({ text: `Could not pull${of(v)}: ${msg(e)}`, ok: false });
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
    const off: string[] = [];
    const stuck: string[] = [];
    let mediaOff = false;

    // Every vault gets its own commit + push, and its own line in the report. A vault
    // that fails must not cancel the others — and must not be quietly folded into a
    // cheerful summary either.
    for (const v of status.vaults) {
      if (!v.remote) {
        stuck.push(v.name);
        steps.push({ text: `No remote set${of(v)} — those notes cannot leave this machine.`, ok: false });
        continue;
      }
      const reach = reachOf(v.remote);
      // Flush whatever the 5s auto-commit debounce has not written yet: it is
      // best-effort and swallows its errors, so never assume it has run.
      try {
        await commit(`auto: ${now}`, v.name);
      } catch (e) {
        steps.push({ text: `Could not commit${of(v)}: ${msg(e)}`, ok: false });
      }
      try {
        const squashed = await push(`backup: ${now}`, v.name);
        if (squashed > 1) steps.push({ text: `Squashed ${squashed} commits into one${of(v)}.`, ok: true });
        steps.push({
          text: `Notes${of(v)} pushed to ${shortDest(v.remote)} — ${left(reach)}.`,
          ok: true,
        });
        (reach === 'remote' ? off : stuck).push(v.name);
      } catch (e) {
        steps.push({ text: `Notes${of(v)} NOT pushed: ${msg(e)}`, ok: false });
        stuck.push(v.name);
      }
    }

    // The tiers are independent — a failed push must not cancel a full backup,
    // and each reports its own fate.
    if (heavy) {
      try {
        await backup();
        steps.push({
          text: `Full backup of every vault, media included → ${shortDest(status.restic_repo ?? '')} — ${left(mediaReach)}.`,
          ok: true,
        });
        mediaOff = mediaReach === 'remote';
      } catch (e) {
        steps.push({ text: `Full backup failed: ${msg(e)}`, ok: false });
      }
    }

    // Name the vaults that did not make it. "Your notes are backed up" while the lab
    // vault sat still is the one sentence this panel must never say.
    verdict =
      (stuck.length === 0
        ? 'Your notes are off this machine.'
        : off.length === 0
          ? 'Your notes are still on this machine.'
          : `Off this machine: ${off.join(', ')}. Still here: ${stuck.join(', ')}.`) +
      ' ' +
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

    <!-- One block per vault: each is its own repo, its own remote, its own audience.
         A single-vault install is a list of one and reads exactly as it always did. -->
    {#each vaults as v (v.name)}
      <div class="vault">
        {#if plural}<h3>{v.name}</h3>{/if}

        <label class="remote">
          <span>{plural ? `The ${v.name} vault's` : "Your notes'"} git remote</span>
          <div class="row">
            <input
              bind:value={remoteDrafts[v.name]}
              onkeydown={(e) => e.key === 'Enter' && canSaveRemote(v) && saveRemote(v)}
              placeholder="git@github.com:you/notes.git"
              spellcheck="false"
              disabled={busy}
            />
            <button onclick={() => saveRemote(v)} disabled={!canSaveRemote(v)}>Save</button>
          </div>
        </label>

        {#if needsIdentity(v)}
          <!-- Asked once, at the only moment it matters: pushing is when your notes
               start carrying your name to someone else, and git history is permanent.
               Per vault, because a vault is an audience — the name on a lab repo need
               not be the one on your personal notes. Anyone who has ever configured git
               never sees this. -->
          <div class="identity">
            <p class="why">
              Every commit you push is signed with a name. Yours isn't set{plural
                ? ` for ${v.name}`
                : ''} — without it, your collaborators can't tell who changed what.
            </p>
            <div class="row">
              <input
                bind:value={nameDrafts[v.name]}
                onkeydown={(e) => e.key === 'Enter' && canSaveRemote(v) && saveRemote(v)}
                placeholder="Your name"
                spellcheck="false"
                disabled={busy}
              />
              <input
                bind:value={emailDrafts[v.name]}
                onkeydown={(e) => e.key === 'Enter' && canSaveRemote(v) && saveRemote(v)}
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
              {#if reachOf(v.remote) === 'unset'}
                <strong>No remote set</strong> — these notes cannot leave this machine yet.
              {:else}
                Notes → <strong>{shortDest(v.remote ?? '')}</strong> — {leaves(reachOf(v.remote))}.
                {#if v.unpushed}
                  <span class="muted">{v.unpushed} commit{v.unpushed === 1 ? '' : 's'} not pushed.</span>
                {/if}
                {#if v.identity}
                  <span class="muted">Signed as {v.identity.name} &lt;{v.identity.email}&gt;.</span>
                {/if}
              {/if}
            </li>
          </ul>
        {/if}

        {#if v.conflicts.length}
          <!-- The one state a user must be told about by name: these notes have both
               versions in them and are waiting for a person. The merge driver keeps the
               markers in the body, so each still opens in the editor. -->
          <p class="error">
            {v.conflicts.length} note{v.conflicts.length === 1 ? '' : 's'} still need you:
            {v.conflicts.join(', ')}. Open each and keep the text you want.
          </p>
        {:else if v.remote_moved}
          <p class="moved">Someone has pushed work you don't have yet.</p>
        {/if}

        {#if v.remote}
          <div class="vault-actions">
            <button onclick={() => bringDown(v)} disabled={busy} class:primary={v.remote_moved}>
              {busy ? 'Working…' : 'Get their changes'}
            </button>
          </div>
        {/if}
      </div>
    {/each}

    {#if steps.length === 0}
      <ul class="promise">
        <li>
          {#if !heavy}
            Media in <code>blobs/</code> (images, PDFs, video) is <strong>not included</strong>.
          {:else}
            Media from <strong>every vault</strong> → <strong>{shortDest(status?.restic_repo ?? '')}</strong>
            — {leaves(mediaReach)}.
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
        {#if plural}
          <span class="muted">
            (one repo for every vault — unlike your git remotes, this does not keep them
            apart)
          </span>
        {/if}
        {#if status && !status.restic_ready}
          <span class="muted">
            (unavailable: set <code>FM_RESTIC_REPO</code> and <code>RESTIC_PASSWORD</code>, then
            restart)
          </span>
        {/if}
      </span>
    </label>

    {#if error}<p class="error">{error}</p>{/if}

    <div class="actions">
      <button onclick={onclose} disabled={busy}>Close</button>
      <button class="primary" onclick={run} disabled={!canRun}>
        {busy
          ? 'Backing up…'
          : `Back up ${plural ? 'every vault' : 'notes'}${heavy ? ' + media' : ''}`}
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
  .vault {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .vault h3 {
    margin: 0;
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--text);
    padding-bottom: var(--space-2);
    border-bottom: 1px solid var(--border);
  }
  .vault-actions {
    display: flex;
    justify-content: flex-end;
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
