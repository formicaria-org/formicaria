<script lang="ts">
  // Bringing notes into formicaria: a name, a folder, and an honest account of what will happen.
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
  import { onMount } from 'svelte';
  import {
    checkImport,
    checkPath,
    cloneVault,
    config as fetchConfig,
    createVault,
    listVaults,
    probeRemote,
    restoreVault,
    runImport,
    setGitCredential,
  } from './ipc';
  import type {
    GitAuth,
    ImportCheck,
    ImportReport,
    PathCheck,
    RemoteProbe,
    VaultInfo,
  } from './types';
  import { describe, historyNote } from './vaultCheck';

  interface Props {
    /** No vaults exist. The app is not usable until this succeeds. */
    firstRun?: boolean;
    /** From the `ping` heartbeat — a capability, not a guess. `null` = not yet answered. */
    git?: boolean | null;
    /** Likewise for restic. `null` = not yet answered; `false` on every phone, and on any
     *  desktop without it installed. */
    restic?: boolean | null;
    oncreated: (vaults: VaultInfo[]) => void;
    oncancel?: () => void;
  }
  let { firstRun = false, git = null, restic = null, oncreated, oncancel }: Props = $props();

  let name = $state('notes');
  let path = $state('~/notes');

  // **Where vaults go, when the user does not choose.** `null` is a desktop: they type a
  // folder, because a vault there is a folder they already have an opinion about. A string is
  // a phone, where there is no `$HOME`, no shell, and no path anyone could meaningfully type —
  // so the form stops asking and the server places the vault inside its own sandbox.
  //
  // The join is the SERVER's: sending an empty path is how the form says "you decide". A
  // browser computing `<root>/<name>` would be one `../` from writing outside the sandbox.
  let vaultRoot = $state<string | null>(null);
  const managed = $derived(vaultRoot !== null);

  onMount(async () => {
    const cfg = await fetchConfig().catch(() => null);
    if (cfg) {
      existing = cfg.vaults;
      vaultRoot = cfg.vault_root;
      // Nothing sensible to prefill on a phone, and the field is not shown there anyway.
      if (cfg.vault_root !== null) path = '';
    }
  });
  let check = $state<PathCheck | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);

  // THREE ways a vault comes into being, and they differ only in what fills the folder
  // before it is registered. Not three components and not three screens: the folder
  // question, the server-owned verdict and the registration are identical in all three, and
  // that shared half is the part that must never drift.
  //
  //   create  — nothing fills it; you start empty
  //   clone   — git fills it from a remote, with history, and you can push back
  //   restore — restic fills it from a backup, with your media, and no history at all
  //
  // The distinction that matters is not the tool, it is **whether what arrives can sync**.
  // Only git carries history, so only git can be a two-way relationship; everything else
  // hands you a copy. Saying that plainly is better than a form that implies otherwise.
  //   import  — another app fills it: a Logseq graph or an Obsidian vault, converted
  //
  // Import is the only one that **converts** rather than moving bytes, and the only one that can
  // also target a vault you already have — so it is the only mode with a destination question.
  type Mode = 'create' | 'clone' | 'restore' | 'import';
  let mode = $state<Mode>('create');
  let url = $state('');
  let gitName = $state('');
  let gitEmail = $state('');
  let repo = $state('');

  // ── import ────────────────────────────────────────────────────────────────────
  let source = $state('');
  /** An existing vault's name, or '' for the new one this form describes. */
  let intoVault = $state('');
  let stubs = $state(false);
  let importCheck = $state<ImportCheck | null>(null);
  let scanning = $state(false);
  let report = $state<ImportReport | null>(null);
  let existing = $state<VaultInfo[]>([]);

  // **Asked before the clone, not after it fails.** A typo, a private repo, and being offline
  // all come out of `git clone` as the same unusable sentence about usernames; these are three
  // different problems with three different next steps.
  let probe = $state<RemoteProbe | null>(null);
  let probing = $state(false);
  // Write-only, always. Nothing ever reads this back out of the server — on a desktop it goes
  // straight into git's own credential helper and formicaria keeps nothing at all.
  let token = $state('');
  let auth = $state<GitAuth | null>(null);
  let authSaved = $state(false);
  let authError = $state<string | null>(null);

  let probeTimer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const u = url;
    clearTimeout(probeTimer);
    probe = null;
    authSaved = false;
    if (mode !== 'clone' || !u.trim()) return;
    probing = true;
    probeTimer = setTimeout(async () => {
      probe = await probeRemote(u).catch(() => null);
      probing = false;
    }, 600); // longer than the path check: this one touches the network
    return () => clearTimeout(probeTimer);
  });

  async function saveToken() {
    authError = null;
    try {
      auth = await setGitCredential(url, token);
      authSaved = true;
      token = ''; // never keep it in the page once it is stored
      probe = await probeRemote(url).catch(() => probe); // re-ask: it should be reachable now
    } catch (e) {
      authError = e instanceof Error ? e.message : String(e);
    }
  }

  // A route the machine cannot take is not offered. `null` means the heartbeat has not
  // answered yet, and we assume capable rather than flashing options away underneath a
  // cursor — the server refuses anyway, with a better sentence than we could write.
  const canClone = $derived(git !== false);
  const canRestore = $derived(restic !== false);

  // Falling back rather than stranding the form in a mode whose fields have vanished — the
  // phone case, where restic will never appear.
  $effect(() => {
    if ((mode === 'clone' && !canClone) || (mode === 'restore' && !canRestore)) mode = 'create';
  });

  // A directory walk, not a stat — so it is debounced longer than the path check, the same way
  // `probeRemote` is because it touches the network. A source path can be anything a person typed,
  // including one with a great many files under it.
  let scanTimer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const [src, into, n, p] = [source, intoVault, name, path];
    clearTimeout(scanTimer);
    if (mode !== 'import' || !src.trim()) {
      importCheck = null;
      return;
    }
    scanning = true;
    scanTimer = setTimeout(async () => {
      importCheck = await checkImport(src, into, n, p).catch(() => null);
      scanning = false;
    }, 600);
    return () => clearTimeout(scanTimer);
  });

  // Importing into a vault that already exists asks no folder question at all — the folder is
  // already decided. Only the *new vault* destination needs a name and a path.
  const importingIntoNew = $derived(mode === 'import' && intoVault === '');

  const described = $derived(describe(check));
  // The SERVER owns this. Never `described.blocking.length === 0` — that would be the
  // browser holding a second opinion, which is how a button enables and then fails.
  //
  // The per-mode fields are checked here only to keep the button honest about what it will
  // attempt; the server validates them again, first, and before it fetches anything.
  const cloneReady = $derived(
    !!url.trim() && !!gitName.trim() && gitEmail.includes('@'),
  );
  // No identity is demanded to restore, and the asymmetry is deliberate: a clone has an
  // audience by definition, a restore has one user by definition — you.
  const restoreReady = $derived(!!repo.trim());
  const modeReady = $derived(
    mode === 'clone' ? cloneReady : mode === 'restore' ? restoreReady : true,
  );
  // In import mode the server answers for the source **and** the destination in one `ok`, so this
  // reads that single verdict rather than ANDing it with a second one of its own — the same rule
  // the other three modes follow with `check.ok`.
  const canCreate = $derived(
    mode === 'import'
      ? !!importCheck?.ok && !busy
      : !!check?.ok && !busy && modeReady,
  );

  // Debounced like the sidebar's search: a keystroke should not be a round trip, but the
  // answer must feel immediate once you stop.
  let timer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const [n, p, m] = [name, path, managed];
    clearTimeout(timer);
    // On a managed install the *name* is the whole input — an empty path is what asks the
    // server to place it — so only a desktop treats a blank folder as nothing to check.
    // Importing into an existing vault creates nothing, so there is no path to check either.
    if ((mode === 'import' && intoVault !== '') || (!m && !p.trim())) {
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
      if (mode === 'import') {
        // The report is the point of the whole exercise — what arrived, what did not, and what
        // was renamed — so it is shown rather than swallowed by the dialog closing. `oncreated`
        // fires when the user dismisses it.
        report = await runImport(source, intoVault, name, path, stubs);
        return;
      }
      oncreated(
        mode === 'clone'
          ? await cloneVault(name, path, url, gitName, gitEmail)
          : mode === 'restore'
            ? await restoreVault(name, path, repo)
            : await createVault(name, path),
      );
    } catch (e) {
      // The server's sentence, verbatim. It is the one that knows what actually happened
      // — including the partial case ("the folder was created, but the list could not be
      // saved"), which we must not smooth over into "couldn't create vault".
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  /** Dismiss the summary. The vault list is re-read rather than assumed: an import into a *new*
   *  vault registered one, and an import into an existing vault changed none. */
  async function finishImport() {
    oncreated(await listVaults().catch(() => existing));
  }

  /** Bytes as something a person reads, for the "and N attachments" line. */
  function mb(bytes: number): string {
    if (bytes < 1_000_000) return `${Math.max(1, Math.round(bytes / 1000))} KB`;
    return `${(bytes / 1_000_000).toFixed(bytes < 10_000_000 ? 1 : 0)} MB`;
  }
</script>

<div class="wrap" class:first={firstRun}>
  {#if report}
    <!-- What actually happened. Shown instead of the form, because every number here is
         something the user would otherwise have to go looking for — and two of them (dangling
         links, renamed properties) are things they can only act on if they are told. -->
    <div class="card" role="status">
      <h1>Imported from {report.format === 'logseq' ? 'Logseq' : 'Obsidian'}</h1>
      <p class="lede">
        <strong>{report.notes}</strong>
        {report.notes === 1 ? 'note' : 'notes'} in <strong>{report.vault}</strong>{#if report.attachments}, with
          <strong>{report.attachments}</strong>
          {report.attachments === 1 ? 'attachment' : 'attachments'}{/if}.
      </p>
      <ul class="says">
        {#if report.links}
          <li class="good">{report.links} links between notes now work.</li>
        {/if}
        {#if report.blocks}
          <li class="good">{report.blocks} block references were replaced by what they said.</li>
        {/if}
        {#if report.alreadyImported}
          <li class="note">
            {report.alreadyImported} were imported before and were left untouched — nothing you
            have edited here was overwritten.
          </li>
        {/if}
        {#if report.dangling}
          <li class="warn">
            {report.dangling} links point at pages that had no file, so they were left as plain
            text{#if report.danglingNames.length}: {report.danglingNames.join(', ')}{/if}.
          </li>
        {/if}
        {#if report.stubs}
          <li class="note">{report.stubs} empty notes were created for those pages.</li>
        {/if}
        {#if report.renamedProperties}
          <li class="note">
            {report.renamedProperties}
            {report.renamedProperties === 1 ? 'property was' : 'properties were'} renamed because the
            name is one a note already uses.
          </li>
        {/if}
        {#each report.leftBehind as l (l.kind)}
          <li class="warn">{l.count} .{l.kind} files were left behind — there is nowhere to put them here.</li>
        {/each}
        {#if report.recorded}
          <li class="good">All of it was saved in one step, so it can be undone in one step.</li>
        {/if}
        {#each report.warnings as w (w)}
          <li class="warn">{w}</li>
        {/each}
      </ul>
      <div class="actions">
        <button type="button" onclick={() => void finishImport()}>Done</button>
      </div>
    </div>
  {:else}
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

    <!-- Three ways in, one form. None of them is a different *kind* of vault — it is the
         same folder question, with the contents arriving from somewhere else first. Options
         the machine cannot take are absent rather than present-and-failing. -->
    <div class="mode" role="group" aria-label="how to add this vault">
      <button
        type="button"
        class:active={mode === 'create'}
        onclick={() => (mode = 'create')}
        disabled={busy}>Start empty</button>
      {#if canClone}
        <button
          type="button"
          class:active={mode === 'clone'}
          onclick={() => (mode = 'clone')}
          disabled={busy}>Join a shared one</button>
      {/if}
      {#if canRestore}
        <button
          type="button"
          class:active={mode === 'restore'}
          onclick={() => (mode = 'restore')}
          disabled={busy}>Restore a backup</button>
      {/if}
      <!-- Absent where there is no folder for the user to point at. On a phone there is no
           $HOME, no shell and no path anyone could type — the same reason the folder field
           disappears there — so offering this would be offering a refusal. -->
      {#if !managed}
        <button
          type="button"
          class:active={mode === 'import'}
          onclick={() => (mode = 'import')}
          disabled={busy}>Import from another app</button>
      {/if}
    </div>

    {#if mode === 'import'}
      <label>
        <span>Source folder</span>
        <input
          bind:value={source}
          placeholder="~/Documents/my-logseq-graph"
          autocomplete="off"
          spellcheck="false"
          autocapitalize="off"
        />
        <small>
          Your Logseq graph or Obsidian vault. It is only ever <strong>read</strong> — nothing is
          changed, moved or deleted there.
        </small>
      </label>

      {#if existing.length}
        <label>
          <span>Put the notes in</span>
          <select bind:value={intoVault} disabled={busy}>
            <option value="">a new vault</option>
            {#each existing as v (v.name)}
              <option value={v.name}>{v.name}</option>
            {/each}
          </select>
          <small>
            A new vault keeps the imported notes separate, which is the easy thing to undo — you
            can forget it again and nothing else changes.
          </small>
        </label>
      {/if}

      <label class="check">
        <input type="checkbox" bind:checked={stubs} disabled={busy} />
        <span>Make a note for pages that are only linked to</span>
        <small>
          Off by default. In Logseq a page exists as soon as something links to it, so a graph
          usually has many with no content — and every one would appear in your board, calendar
          and timeline. Left off, those links stay as plain text.
        </small>
      </label>
    {/if}

    {#if !(mode === 'import' && intoVault !== '')}
      <label>
        <span>Name</span>
        <input bind:value={name} placeholder="notes" autocomplete="off" spellcheck="false" />
        <small>What you'll call it here. Anything you like.</small>
      </label>
    {/if}

    {#if mode === 'clone'}
      <label>
        <span>Repo URL</span>
        <input
          bind:value={url}
          placeholder="https://github.com/you/notes.git"
          autocomplete="off"
          spellcheck="false"
          autocapitalize="off"
        />
        <small>
          Cloned with your existing git credentials.
        </small>
      </label>

      {#if probing}
        <p class="lede small">Checking the repo…</p>
      {:else if probe?.state === 'reachable'}
        <p class="lede small good-line">✓ {probe.detail}</p>
      {:else if probe?.state === 'unreachable' && probe.detail}
        <!-- git's own words, deliberately. We did not recognise this, and inventing a
             friendlier sentence would mean guessing — which is how someone ends up
             configuring credentials for a URL they simply mistyped. -->
        <p class="lede small caveat"><code>{probe.detail}</code></p>
      {:else if probe?.state === 'needs_auth'}
        <div class="auth">
          <p class="lede small">{probe.detail}</p>
          {#if authSaved}
            <p class="lede small good-line">✓ Saved. Try the URL again.</p>
          {:else}
            <label>
              <span>Access token</span>
              <input
                type="password"
                bind:value={token}
                placeholder="github_pat_…"
                autocomplete="off"
                spellcheck="false"
                autocapitalize="off" />
              <!-- The advice that actually limits a leak. A GitHub token is a *bearer*
                   token — it is not tied to a device, and anyone holding the string can use
                   it from anywhere — so scope and expiry are the only things that bound the
                   damage. Said here because this is the one moment it is actionable. -->
              <small>
                Use a <strong>fine-grained</strong> token limited to this one repository, with
                contents read/write and an expiry date. Tokens are not tied to a device: anyone
                who has one can use it from anywhere, so a narrow token is the protection.
              </small>
            </label>
            {#if probe.helper_is_plaintext}
              <p class="lede small caveat">
                Heads up: git on this machine uses the <code>store</code> helper, which keeps
                credentials as <strong>plain text</strong> in <code>~/.git-credentials</code>.
                That is where this token will go.
              </p>
            {/if}
            <div class="actions">
              <button type="button" onclick={() => void saveToken()} disabled={!token.trim()}>
                Save token
              </button>
            </div>
          {/if}
          {#if authError}<p class="error" role="alert">{authError}</p>{/if}
        </div>
      {/if}

      <!-- Required, and the copy says why. This is the one place where leaving the committer
           unset is not merely untidy: a shared vault on the placeholder attributes everyone's
           commits to the same fake person, and git history is not something you fix later. -->
      <label>
        <span>Your name</span>
        <input bind:value={gitName} placeholder="Ada Lovelace" autocomplete="off" />
        <small>Signs every commit you make here.</small>
      </label>

      <label>
        <span>Your email</span>
        <input
          bind:value={gitEmail}
          placeholder="ada@example.org"
          autocomplete="off"
          spellcheck="false"
          autocapitalize="off"
        />
        <small>How your collaborators will see your changes attributed.</small>
      </label>
    {/if}

    {#if mode === 'restore'}
      <label>
        <span>Backup repository</span>
        <input
          bind:value={repo}
          placeholder="~/backups/notes-repo"
          autocomplete="off"
          spellcheck="false"
          autocapitalize="off"
        />
        <small>
          The restic repository your vault was backed up to. Unlocked with the backup password
          this machine holds — set it under <strong>Back up</strong>, or set
          <code>RESTIC_PASSWORD</code> in the environment, which wins where both exist.
        </small>
      </label>

      <!-- Said before the button, not after the restore. What comes back is genuinely less
           than what a clone brings, and a user who expected their history would find it
           missing at the worst possible moment: after their old machine is gone. -->
      <p class="lede caveat">
        Restoring brings back your <strong>notes and attachments</strong> — not your history.
        Backups snapshot the vault's own folders, so there is no <code>.git</code> in them and
        nothing to pull from or push to. You'll get a working vault you own outright; turn on
        history later if you want one.
      </p>
    {/if}

    {#if mode === 'import' && intoVault !== ''}
      <!-- No folder question: the destination already has one. -->
    {:else if managed}
      <!-- No folder question, because there is no folder to choose. The location is stated
           rather than hidden: files-as-truth means "where is my file" must always have an
           answer, even when the answer is somewhere you cannot browse to. -->
      <p class="lede caveat">
        Kept in formicaria's own storage on this device — <code>{vaultRoot}</code> — where no
        other app can read or write. Uninstalling formicaria deletes it, so give a vault you
        care about a remote or a backup.
      </p>
    {:else}
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
    {/if}

    <!-- Everything below is a promise about someone's filesystem. Each line is a fact the
         server reported, never an inference we made. -->
    <ul class="says" aria-live="polite">
      {#if mode === 'import'}
        <!-- The size of the import, stated **before** the button is pressed. That is what makes
             one blocking call acceptable for a job this long: nobody is surprised by how much
             they asked for. Every number is the server's, not a guess made here. -->
        {#if scanning}
          <li class="note">Looking at that folder…</li>
        {:else if importCheck?.problem}
          <li class="bad">{importCheck.problem}</li>
        {:else if importCheck?.ok}
          <li class="good">
            {importCheck.label}: {importCheck.pages + importCheck.journals} pages{#if importCheck.journals}
              (including {importCheck.journals} daily {importCheck.journals === 1 ? 'note' : 'notes'}){/if}{#if importCheck.attachments},
              and {importCheck.attachments} attachments — {mb(importCheck.attachmentBytes)}{/if}.
          </li>
          {#if importCheck.pages + importCheck.journals > 2000}
            <li class="warn">
              That is a lot of notes. The app will be busy while it works, and lists this long are
              slower to draw than the ones it was built for.
            </li>
          {/if}
          {#each importCheck.leftBehind as l (l.kind)}
            <li class="warn">
              {l.count} .{l.kind}
              {l.count === 1 ? 'file' : 'files'} will be left behind — there is nowhere to put
              {l.count === 1 ? 'it' : 'them'} here.
            </li>
          {/each}
          <li class="note">
            Your notes stay exactly as they are in {importCheck.label}. This makes a copy; the two
            do not stay in step afterwards.
          </li>
        {/if}
      {:else}
      {#each described.blocking as msg (msg)}
        <li class="bad">{msg}</li>
      {/each}
      {#each described.warnings as msg (msg)}
        <li class="warn">{msg}</li>
      {/each}
      {#if check?.ok && described.warnings.length === 0}
        <li class="good">
          {mode === 'clone'
            ? 'Ready to join.'
            : mode === 'restore'
              ? 'Ready to restore.'
              : 'Ready to create.'}
        </li>
      {/if}
      <!-- Suppressed while restoring: the caveat above already says this vault arrives with
           no history, and following it with git's general availability note would read as a
           contradiction of the sentence directly above it. -->
      {#if git !== null && mode !== 'restore'}
        <li class="note">{historyNote(git)}</li>
      {/if}
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
        {#if mode === 'import'}
          {busy ? 'Importing…' : 'Import notes'}
        {:else if mode === 'clone'}
          {busy ? 'Joining…' : 'Join vault'}
        {:else if mode === 'restore'}
          {busy ? 'Restoring…' : 'Restore vault'}
        {:else}
          {busy ? 'Creating…' : 'Create vault'}
        {/if}
      </button>
    </div>
  </form>
  {/if}
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
  .small {
    font-size: 0.85rem;
  }
  .good-line {
    color: var(--text, inherit);
  }
  .auth {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-2, 6px);
  }
  .caveat {
    font-size: 0.9rem;
    padding: 8px 10px;
    border-left: 2px solid var(--border);
    background: var(--surface, transparent);
    border-radius: 0 var(--radius-2, 6px) var(--radius-2, 6px) 0;
  }
  .mode {
    display: flex;
    flex-wrap: wrap; /* three options do not fit one phone-width row */
    gap: var(--space-2);
  }
  .mode button {
    flex: 1 1 8rem;
    /* The touch target both platform guidelines ask for. This row decides what the whole
       form means, and it is the first thing a thumb lands on. */
    min-height: 2.75rem;
    padding: 6px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-2, 6px);
    background: transparent;
    color: var(--text-muted);
    font-size: var(--text-sm);
    cursor: pointer;
  }
  .mode button.active {
    background: var(--surface-elevated);
    color: var(--text);
    border-color: var(--text-muted);
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
  /* A checkbox reads as "box, then what it does" — not as a stacked field like the text
     inputs above it, where the label sits over the control. */
  .check {
    display: grid;
    grid-template-columns: auto 1fr;
    align-items: baseline;
    gap: 4px 8px;
  }
  .check input {
    width: auto;
    justify-self: start;
  }
  .check small {
    grid-column: 2;
  }
  select {
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-2, 6px);
    background: var(--bg);
    color: var(--fg);
    font: inherit;
  }
</style>
