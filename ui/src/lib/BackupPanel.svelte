<script lang="ts">
  // Backing up in two tiers, and saying which one you actually got.
  //
  // Light (the default): commit + push the notes. Text only — small, plain, and
  // authenticated by whatever ssh-agent or credential helper the user already
  // has, so this app stores no secret.
  // Heavy (tick to include): restic, blobs and all.
  //
  // The panel's whole job is to never overstate. It says what each tier will and
  // will not carry *before* you act, and afterwards reports each tier's real
  // outcome — including whether the data left this machine at all, which a local
  // path for a remote or a restic repo quietly does not.
  //
  // **It is a list, not a form**, and *both* tiers are per vault. Git because one vault
  // is one repo, one remote, one collaborator list. Restic for the same shape of reason:
  // a restic repo is per repository, so a set of vaults needs one each — there is no
  // single media destination they could share. So each vault gets its own destination,
  // identity, unpushed count, "someone pushed", and its own snapshot. Collapsing any of
  // that into one "Back up" that silently meant the first vault is exactly the
  // overstatement this panel exists to prevent — hence a vault with no restic repo is
  // named, not skipped in silence.
  import { onMount } from 'svelte';
  import { backup, backupStatus, commit, forgetVault, pull, setGitRemote } from './ipc';
  import { syncVault, syncFor } from './sync.svelte';
  import { conflictLabels } from './conflictLabel';
  import { reachOf, shortDest } from './destination';
  import type { BackupStatus, VaultStatus } from './types';

  // `onnewvault` because this panel is already "a list, not a form" — the one surface in
  // the app that is *about the set of vaults*, which makes it where you add one. (The
  // sidebar is search-first with New note / New board; those are note gestures, and a
  // vault is not a note.)
  let { onclose, onnewvault }: { onclose: () => void; onnewvault: () => void } = $props();

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

  // Git is a capability this machine may simply not have. Without it every vault reads
  // `remote: null, identity: null`, which looks exactly like "not set up yet" — so the
  // panel would cheerfully invite you to configure a tier that cannot run.
  const noGit = $derived(!!status && !status.git);
  const vaults = $derived(status?.vaults ?? []);
  // Tickable if *anyone* can take media. Vaults without a restic repo are not a reason to
  // grey out the ones that have one — they are a reason to say their media stayed put.
  const anyRestic = $derived(vaults.some((v) => v.restic_ready));
  // "restic isn't installed" and "you haven't given this vault a repo" are different
  // problems with different fixes, so never say one when you mean the other.
  const noRestic = $derived(!!status && !status.restic);
  // Something to push somewhere. A vault with no remote isn't a failure, it just has
  // nowhere to go yet.
  const canRun = $derived(!busy && !noGit && vaults.some((v) => !!v.remote));
  // Only the people git has never met get asked, and only about the vault they are
  // sharing: a vault is an audience, so the name on a lab repo need not be the one on
  // your personal notes.
  const needsIdentity = (v: VaultStatus) => v.identity === null;
  const canSaveRemote = (v: VaultStatus) =>
    !busy &&
    !noGit &&
    !!remoteDrafts[v.name]?.trim() &&
    (!needsIdentity(v) || (!!nameDrafts[v.name]?.trim() && !!emailDrafts[v.name]?.trim()));

  const msg = (e: unknown) => (e instanceof Error ? e.message : String(e));
  const leaves = (r: string) => (r === 'remote' ? 'leaves this machine' : 'stays on this machine');
  const left = (r: string) => (r === 'remote' ? 'off this machine' : 'still on this machine');
  // A single vault has no boundary to talk about, so don't name it at every turn.
  const plural = $derived(vaults.length > 1);
  const of = (v: VaultStatus) => (plural ? ` (${v.name})` : '');

  /// Which vault has been clicked once. A two-step, because it changes what the app shows you and a
  /// single misclick in a list of vaults should not.
  let confirmForget = $state<string | null>(null);
  /// The result of a removal, shown **beside the control that did it** rather than in the panel's
  /// `verdict` line — that one only renders inside a backup run's step list, so a message put there
  /// is a message nobody sees.
  let forgetNote = $state<string | null>(null);
  async function doForget(name: string) {
    busy = true;
    try {
      const r = await forgetVault(name);
      confirmForget = null;
      // Says what stayed. The count is the whole point: nobody should have to wonder whether
      // removing a vault from a list deleted their notes.
      forgetNote =
        r.notes > 0
          ? `Removed “${r.forgotten}”. Its ${r.notes} note${r.notes === 1 ? '' : 's'} are still on disk at ${r.path}.`
          : `Removed “${r.forgotten}” — it was empty. Nothing was deleted.`;
      error = null;
      await load();
    } catch (e) {
      forgetNote = String(e);
    } finally {
      busy = false;
    }
  }

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
      // Commit first (git will not merge over uncommitted edits) — but a commit that
      // committed *nothing* because the vault is mid-merge must stop us here rather than fall
      // into a pull that will only refuse, with git's wording instead of ours.
      const c = await commit(`auto: ${new Date().toISOString()}`, v.name).catch(() => null);
      const blocked = !!c && !c.committed && c.conflicts.length > 0;
      if (blocked) {
        // Not an early return: `busy = false` lives after this block, not in a `finally`,
        // so returning here would leave the panel frozen.
        steps.push({
          text: `${c!.conflicts.length} note${c!.conflicts.length === 1 ? '' : 's'} in ${v.name} still need you: ${(await conflictLabels(c!.conflicts)).join(', ')}. Nothing is being committed until they are settled — see “Needs resolution” in the Collaboration view, which says what to do for each one.`,
          ok: false,
        });
      }
      const r = blocked ? { merged: 0, conflicts: [] } : await pull(v.name);
      if (blocked) {
        // nothing further to report; the line above is the answer
      } else if (r.conflicts.length) {
        steps.push({
          text: `Merged${of(v)}, but ${r.conflicts.length} note${r.conflicts.length === 1 ? '' : 's'} need you: ${(await conflictLabels(r.conflicts)).join(', ')}. See “Needs resolution” in the Collaboration view: some are resolved by editing the note, and some only by choosing a side.`,
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
    const mediaOff: string[] = [];
    const noMedia: string[] = [];

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
      // `syncVault` is commit → push, and — if the remote moved while you were writing —
      // pull, merge, push once more. That last part is the difference between this button
      // working and this button telling you "the remote has changes you don't have — pull
      // first, then back up" and making you do it by hand. Exactly one retry: a loop is how
      // a rejected push becomes an invisible one.
      const phase = await syncVault(v.name, `backup: ${now}`, undefined);
      const s = syncFor(v.name);
      if (s.merged > 0) {
        steps.push({ text: `Brought down ${s.merged} change(s)${of(v)} first.`, ok: true });
      }
      if (phase === 'synced') {
        steps.push({
          text: `Notes${of(v)} pushed to ${shortDest(v.remote)} — ${left(reach)}.`,
          ok: true,
        });
        (reach === 'remote' ? off : stuck).push(v.name);
      } else if (phase === 'conflicts') {
        // Deliberately not pushed. Those notes hold both versions in their bodies, and
        // publishing conflict markers as content is worse than not publishing.
        steps.push({
          text:
            `Notes${of(v)} NOT pushed — ${s.conflicts.length} note(s) came back with ` +
            `conflicting edits and need you first: ${(await conflictLabels(s.conflicts)).join(', ')}`,
          ok: false,
        });
        stuck.push(v.name);
      } else {
        steps.push({ text: `Notes${of(v)} NOT pushed: ${msg(s.error)}`, ok: false });
        stuck.push(v.name);
      }
    }

    // The media tier, per vault and for the same reason git is: a restic repo is per
    // repository, so each vault either has one or its media has nowhere to go. Back up
    // the ones that can and **name the ones that can't** — silently skipping them is the
    // failure this panel exists to prevent. Independent of the git tier: a failed push
    // must not cancel a snapshot.
    if (heavy) {
      for (const v of status.vaults) {
        if (!v.restic_ready) {
          noMedia.push(v.name);
          steps.push({
            text: v.restic_repo
              ? `Media${of(v)} NOT backed up: RESTIC_PASSWORD isn't set.`
              : `Media${of(v)} NOT backed up — no restic repo configured for it.`,
            ok: false,
          });
          continue;
        }
        const reach = reachOf(v.restic_repo);
        try {
          await backup(v.name);
          steps.push({
            text: `Media${of(v)} → ${shortDest(v.restic_repo ?? '')} — ${left(reach)}.`,
            ok: true,
          });
          if (reach === 'remote') mediaOff.push(v.name);
        } catch (e) {
          steps.push({ text: `Media${of(v)} backup failed: ${msg(e)}`, ok: false });
          noMedia.push(v.name);
        }
      }
    }

    // Name the vaults that did not make it. "Your notes are backed up" while the lab
    // vault sat still is the one sentence this panel must never say.
    // Name the vaults that did not make it, on both tiers. "Your notes are backed up"
    // while one vault sat still is the one sentence this panel must never say.
    verdict =
      (stuck.length === 0
        ? 'Your notes are off this machine.'
        : off.length === 0
          ? 'Your notes are still on this machine.'
          : `Notes off this machine: ${off.join(', ')}. Still here: ${stuck.join(', ')}.`) +
      ' ' +
      (!heavy
        ? 'Your media was not included.'
        : noMedia.length === 0
          ? mediaOff.length
            ? 'Your media is off this machine.'
            : 'Your media is backed up, but still on this machine.'
          : mediaOff.length === 0
            ? `No media was backed up (${noMedia.join(', ')} ${noMedia.length === 1 ? 'has' : 'have'} no restic repo).`
            : `Media off this machine: ${mediaOff.join(', ')}. Not backed up: ${noMedia.join(', ')}.`);
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
    <div class="panel-head">
      <h2>Back up</h2>
      <button class="new-vault" onclick={onnewvault} disabled={busy}>New vault…</button>
    </div>

    {#if noGit}
      <!-- Say what is actually wrong. Every vault below reports no remote and no identity,
           which reads as "unconfigured" — but nothing here can work until git exists, and
           inviting someone to type a remote into it would be a lie. The notebook itself is
           unaffected, and that is worth saying in the same breath. -->
      <p class="error">
        <strong>git isn't installed on this machine.</strong> Your notes are safe — they're
        Markdown files on disk and the app works normally — but nothing on this panel can
        run without git: no history, no backup, no sharing. Install git and reopen.
      </p>
    {/if}

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

        <!-- **Stop showing me this vault.** Deliberately last, small, and worded as what it is:
             unregistering, not deleting. The notes stay on disk and the message says how many and
             where — "forget" and "destroy" are different verbs and only one is reversible. This
             exists because three commands could create a vault and none could remove one, so the
             empty default the phone auto-creates was unremovable from inside the app. -->
        <div class="forget">
          {#if forgetNote && confirmForget !== v.name}
            <p class="forget-note">{forgetNote}</p>
          {/if}
          {#if confirmForget === v.name}
            <span class="muted">
              Remove “{v.name}” from this device's list? Its files stay where they are.
            </span>
            <button type="button" onclick={() => doForget(v.name)} disabled={busy}>
              Yes, remove it
            </button>
            <button type="button" onclick={() => (confirmForget = null)} disabled={busy}>
              Cancel
            </button>
          {:else}
            <button type="button" class="quiet" onclick={() => (confirmForget = v.name)} disabled={busy}>
              Remove this vault from the list…
            </button>
          {/if}
        </div>

        {#if v.conflicts.length}
          <!-- The one state a user must be told about by name: these notes have both
               versions in them and are waiting for a person. The merge driver keeps the
               markers in the body, so each still opens in the editor. -->
          <p class="error">
            {v.conflicts.length} note{v.conflicts.length === 1 ? '' : 's'} still need you:
            {v.conflicts.join(', ')}. Resolve them in “Needs resolution” (Collaboration) — a note
            deleted on one device and edited on the other has no text to merge, so it needs you to
            pick a side.
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
            {#each vaults as v (v.name)}
              {#if v.restic_ready}
                <div>
                  Media{of(v)} → <strong>{shortDest(v.restic_repo ?? '')}</strong> —
                  {leaves(reachOf(v.restic_repo))}.
                </div>
              {:else}
                <div>
                  Media{of(v)} <strong>will not be backed up</strong> —
                  {v.restic_repo ? 'RESTIC_PASSWORD isn\'t set' : 'no restic repo for it'}.
                </div>
              {/if}
            {/each}
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

    <label class="heavy" class:disabled={!anyRestic}>
      <input type="checkbox" bind:checked={heavy} disabled={busy || !anyRestic} />
      <span>
        Include media — restic backup
        {#if noRestic}
          <span class="muted">
            (unavailable: restic isn't installed on this machine — media backup is an
            optional feature, and your notes don't need it)
          </span>
        {:else if status && !anyRestic}
          <span class="muted">
            (unavailable: no vault has a <code>restic</code> repo in your vault list, or
            <code>RESTIC_PASSWORD</code> isn't set — then restart)
          </span>
        {:else if plural && vaults.some((v) => !v.restic_ready)}
          <span class="muted">
            (only the vaults with a restic repo of their own: {vaults
              .filter((v) => v.restic_ready)
              .map((v) => v.name)
              .join(', ')})
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
  /* The remove control: quiet by default, because it is the one action here that changes what the
     app shows you. Its own sizing rather than `.icon-btn` — every `.icon-btn` is hidden in the narrow
     layouts, which is how a control that must stay reachable on a phone silently vanishes. */
  .forget {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.75rem;
  }
  .forget button {
    min-height: 2.5rem;
    padding: 0 0.8rem;
    border-radius: 0.4rem;
    border: 1px solid var(--border);
    background: var(--surface);
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  .forget-note {
    flex-basis: 100%;
    margin: 0;
    font-size: 0.85rem;
    color: var(--text-muted);
  }
  .forget button.quiet {
    border-color: transparent;
    background: none;
    color: var(--text-muted);
    text-decoration: underline;
  }
  .forget button:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .panel-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
  }
  .new-vault {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--fg);
    padding: 4px 10px;
    border-radius: var(--radius-2, 6px);
    font-size: 0.85rem;
    cursor: pointer;
  }

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
