<script lang="ts">
  // Backing up in two tiers, and saying which one you actually got.
  //
  // Light (the default): commit + push the notes. Text — plus, where the vault sets a
  // `git_assets_max`, attachments at or under it, which is why the promise line reads the
  // limit instead of claiming "notes only" for every vault.
  // Heavy (tick to include): an encrypted restic snapshot of this vault's *data* — the notes
  // directory and `blobs/`. Not "media", which is what this panel used to call it: the
  // snapshot carries the notes too, and never the vault root's own files or `.git`.
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
  import { onMount, type Snippet } from 'svelte';
  import {
    backupStatus,
    lastCommits,
    backupLatest,
    forgetVault,
    recoverableVaults,
    createVault,
    gitAuth,
    setGitCredential,
    setGitRemote,
    setResticPassword,
    setResticRepo,
  } from './ipc';
  import { plainError, syncFor } from './sync.svelte';
  import { reachOf, shortDest } from './destination';
  import { GIT_ASSETS_CEILING, humanSize } from './size';
  import type { VaultInfo, Recoverable } from './types';

  /// A vault as this panel needs it: the network facts from `backup_status`, and the cheap ones
  /// from the vault list. Both used to arrive on `VaultStatus`, which meant the slow command
  /// re-described a vault it had no special knowledge of.
  type PanelVault = VaultStatus & Pick<VaultInfo, 'identity' | 'git_assets_max' | 'restic_repo'>;
  import { labelFor, refreshVaults, vaultList } from './vaults.svelte';
  import type { BackupStatus, GitAuth, LastCommit, LatestBackup, VaultStatus } from './types';

  // **Not a window any more** (`decisions.md#ui`, 2026-10-15): this is the backup half of
  // Settings → Vaults & backup, drawn inside that category's section. Settings owns the heading, the
  // *New vault…* button and closing; `vaultRows` is Settings' own per-vault rows (name, folder,
  // attachment limit, consent), drawn at the top of each vault's block so a vault is one block.
  /// `onvaults` fires whenever this panel changes **which vaults exist** — a removal or a recovery.
  /// Without it nothing refetched `list_vaults`, so after a removal the vault filter, the "Create
  /// in" picker and the label registry all still described the world before it, until the app was
  /// relaunched. On a phone that is a long time, and it is what turned one mistaken removal into
  /// "my notes are gone" (2026-09-08).
  let { onvaults, vaultRows }: { onvaults?: () => void; vaultRows?: Snippet<[string]> } = $props();

  let status = $state<BackupStatus | null>(null);
  // Per vault, keyed by name — a shared draft would put your lab remote in your
  // personal vault the moment you looked away.
  let remoteDrafts = $state<Record<string, string>>({});
  let nameDrafts = $state<Record<string, string>>({});
  let emailDrafts = $state<Record<string, string>>({});
  // **The credential half of "your notes can leave this machine".** A vault created here and
  // later given an `https://` remote had a URL, an identity, and nowhere to put a token — the
  // field existed only in the clone form, which this user never went through. So Back up failed
  // at the push with an authentication error and the panel offered nothing to do about it.
  //
  // Per vault, like every other draft here, and cleared the instant it is stored: a token is a
  // bearer credential and has no business outliving the request that carries it.
  let tokenDrafts = $state<Record<string, string>>({});
  // What this machine can do about credentials for each vault's remote, from `git_auth` — which
  // is a **local** question (which helper is configured, do we already hold something for this
  // host) and costs no network. Deliberately not a `probe_remote`: `backup_status` already does
  // one `ls-remote` per vault, and a second round trip per vault, per panel open, to learn
  // something git can answer from disk would be a poor trade.
  let auth = $state<Record<string, GitAuth | null>>({});
  /// Per vault, and `null` when the call itself failed — see `loadLatest`.
  let latest = $state<Record<string, LatestBackup | null>>({});

  /// **When each vault last saved anything to git.** The panel half of the toolbar's quiet chip:
  /// the chip says *which* vault has gone silent and this says it per vault, beside the remote and
  /// the unpushed count, where somebody who came here to find out can read all three together.
  ///
  /// Stated for every vault, not only the quiet ones. A threshold decides when to *interrupt*
  /// someone; a panel they opened on purpose should answer the question it was opened with.
  let saves = $state<LastCommit[]>([]);
  const savedAt = (name: string) => saves.find((s) => s.vault === name)?.last_commit ?? null;
  let authSaved = $state<Record<string, boolean>>({});
  // **The media tier, configurable at last.** Both halves of it used to live outside the app: the
  // repo was a key you hand-edited into `vaults.json` (Settings said so, verbatim: *"there is no
  // UI for it"*) and the password was an environment variable whose documented answer was a
  // launcher you had edited yourself. Every other tier of backup is set up here; this one asked
  // the user to be a system administrator first.
  let resticDrafts = $state<Record<string, string>>({});
  // One for every repo, so it is not per vault — a second password would only multiply the places
  // a secret lives, and losing any of them loses the backups it unlocks.
  let passwordDraft = $state('');
  let passwordSaved = $state(false);
  let busy = $state(false);
  let error = $state<string | null>(null);

  // Git is a capability this machine may simply not have. Without it every vault reads
  // `remote: null, identity: null`, which looks exactly like "not set up yet" — so the
  // panel would cheerfully invite you to configure a tier that cannot run.
  const noGit = $derived(!!status && !status.git);
  /// **One vault, from its two producers.** `backup_status` answers what only it can — the remote,
  /// what is unpushed, whether someone has pushed, which notes conflict — and pays a network
  /// `ls-remote` per vault to do it. Everything else about a vault (its committer, its attachment
  /// limit, its snapshot repo) is a cheap local read and belongs on the vault list, which is where
  /// it is now produced. It used to be reported by *both*, refreshed at different moments, so the
  /// two screens could disagree about the same field. Merged here, once, rather than at eighteen
  /// call sites.
  /// **Vaults in the list that could not be opened** — a folder deleted, moved or unreadable. Not
  /// in `backup_status`, which only knows vaults that opened, so without this a vault whose folder
  /// went to the Trash vanished from this screen along with the one button that removes it.
  const unopened = $derived(
    status ? (vaultList() ?? []).filter((i) => !status!.vaults.some((v) => v.name === i.name)) : [],
  );
  const vaults = $derived(
    (status?.vaults ?? []).map((s): PanelVault => {
      const info = vaultList()?.find((v) => v.name === s.name);
      return {
        ...s,
        identity: info?.identity ?? null,
        git_assets_max: info?.git_assets_max ?? null,
        restic_repo: info?.restic_repo ?? null,
      };
    }),
  );
  // "restic isn't installed" and "you haven't given this vault a repo" are different
  // problems with different fixes, so never say one when you mean the other.
  const noRestic = $derived(!!status && !status.restic);
  // Only the people git has never met get asked, and only about the vault they are
  // sharing: a vault is an audience, so the name on a lab repo need not be the one on
  // your personal notes.
  const needsIdentity = (v: PanelVault) => v.identity === null;
  // **Only an `https://` remote can use a token.** An `ssh://`/`git@` remote authenticates with
  // the key in your agent, and offering a token field for one would be inviting a user to solve
  // a problem they do not have with a credential that will never be consulted.
  const usesToken = (v: PanelVault) => !!v.remote && /^https?:\/\//i.test(v.remote);
  // Ask for a token only where pasting one would change something: an HTTPS remote, git present,
  // and nothing already stored for it. A machine whose helper already has the credential is
  // finished, and says so instead of showing an empty box.
  const needsToken = (v: PanelVault) =>
    usesToken(v) && !noGit && !!auth[v.name] && !auth[v.name]!.have_credential;
  /// **A stored credential that the remote rejects is not a credential.** `needsToken` asks only
  /// whether one is *present*, so a token that has expired or been revoked left the field hidden
  /// while the push failed on it — the panel said "check the token for this remote" and offered no
  /// way to change it. Reported from the phone, 2026-09-08, with three commits stuck behind it.
  /// Session-scoped on purpose: this is the answer to a push that just failed, and it appears in
  /// the same breath as the reason.
  const authRejected = (v: PanelVault) => {
    const st = syncFor(v.name);
    return (
      usesToken(v) &&
      !noGit &&
      st.phase === 'failed' &&
      /authentication|auth failed|401|403/i.test(st.error ?? '')
    );
  };
  const canSaveRemote = (v: PanelVault) =>
    !busy &&
    !noGit &&
    !!remoteDrafts[v.name]?.trim() &&
    (!needsIdentity(v) || (!!nameDrafts[v.name]?.trim() && !!emailDrafts[v.name]?.trim()));

  const msg = (e: unknown) => (e instanceof Error ? e.message : String(e));

  /// When a backlog is big enough to be worth mentioning. Mirrors `sync.svelte.ts`'s own budget,
  /// and is used *only* to decide whether to say something — never to decide behaviour. The
  /// deciding lives in `syncVault`, which every backup path goes through, because putting it here
  /// is precisely the bug this replaces: the option sat in this panel while the button most people
  /// press is the one in the toolbar.
  const OFFER_OVER = 24_000_000;
  const leaves = (r: string) => (r === 'remote' ? 'leaves this machine' : 'stays on this machine');
  // **What the git tier actually carries, per vault.** Not a decoration: this panel used to say
  // "media is not included" flatly, which is false for any vault with a `git_assets_max` — its
  // blobs at or under the limit are `git add -f`'d into the same commit and pushed with the
  // notes. The limit is set in Settings ("Send attachments under"); saying so here is what stops
  // the two panels contradicting each other.
  //
  // **The *effective* limit, never the file's raw number.** A `vault.json` may ask for more than
  // `GIT_ASSETS_CEILING`, and the staging walk clamps it — so quoting the raw value here would
  // promise a push that carries more than it does. That is the same class of overstatement this
  // line was written to remove.
  const effectiveMax = (v: PanelVault) =>
    v.git_assets_max ? Math.min(v.git_assets_max, GIT_ASSETS_CEILING) : null;
  const carries = (v: PanelVault) => {
    const max = effectiveMax(v);
    return max ? `notes, and attachments up to ${humanSize(max)}` : 'notes only';
  };
  // The password is the app's now, and the environment variable is the override rather than the
  // mechanism. Naming `RESTIC_PASSWORD` as *the* thing that is missing sent people to a launcher
  // script to fix something the field above this line fixes.
  const noPassword = 'no backup password is set on this machine';
  // A single vault has no boundary to talk about, so don't name it at every turn.
  const plural = $derived(vaults.length > 1);
  /// **The label, not the name.** This said `v.name`, which is the local identity — the string a
  /// command takes and, on a phone, the folder a vault lives in. Every other surface shows the
  /// repository behind the remote, so a vault appeared here under a name it had nowhere else: the
  /// owner's laptop says `vault` where the phone says `notes`. The App summary was fixed for
  /// exactly this on 2026-09-08 and this copy was missed, which is what happens when one fact has
  /// two producers. See `vaults.svelte.ts`.
  const of = (v: PanelVault) => (plural ? ` (${labelFor(v.name)})` : '');

  /// Which vault has been clicked once. A two-step, because it changes what the app shows you and a
  /// single misclick in a list of vaults should not.
  let confirmForget = $state<string | null>(null);
  /// The result of a removal, shown **beside the control that did it** rather than in the panel's
  /// `verdict` line — that one only renders inside a backup run's step list, so a message put there
  /// is a message nobody sees.
  let forgetNote = $state<string | null>(null);
  /// **Vault folders on this device that are not in the list.** The way back from a removal nobody
  /// meant — and the reason it is a *list* rather than a name to retype: on a phone the name is the
  /// address (`resolve_path`), and every other screen shows a vault's label rather than its name,
  /// so the one identity needed to recover was the one nothing displayed. Reported 2026-09-08, by
  /// someone who had just removed the wrong of two.
  let recoverable = $state<Recoverable[]>([]);
  /// Its own message, not `forgetNote`: that one is rendered inside the per-vault loop, so a single
  /// sentence appears once per vault on the screen. Shown **outside** the section below, because
  /// adding the last folder back empties the list and would take the confirmation with it — the
  /// same trap `authSaved` exists to avoid one block up.
  let recoveredNote = $state<string | null>(null);
  async function loadRecoverable() {
    recoverable = await recoverableVaults().catch(() => []);
  }
  async function takeBack(r: Recoverable) {
    busy = true;
    try {
      await createVault(r.name, r.path);
      onvaults?.();
      recoveredNote = `Added “${r.name}” back with its ${r.notes} note${r.notes === 1 ? '' : 's'}.`;
      error = null;
      await load();
    } catch (e) {
      forgetNote = String(e);
    } finally {
      busy = false;
    }
  }
  async function doForget(name: string) {
    busy = true;
    try {
      const r = await forgetVault(name);
      confirmForget = null;
      // Says what stayed. The count is the whole point: nobody should have to wonder whether
      // removing a vault from a list deleted their notes.
      onvaults?.();
      forgetNote =
        r.notes > 0
          ? `Removed “${r.forgotten}”. Its ${r.notes} note${r.notes === 1 ? '' : 's'} are still on disk at ${r.path}.`
          : `Removed “${r.forgotten}” — it was empty. Nothing was deleted.`;
      error = null;
      await load();
    } catch (e) {
      recoveredNote = String(e);
    } finally {
      busy = false;
    }
  }

  onMount(load);

  async function load() {
    try {
      status = await backupStatus();
      // The cheap half of a vault, from its own producer.
      await refreshVaults();
      await loadRecoverable();
      for (const v of vaults) {
        remoteDrafts[v.name] ??= v.remote ?? '';
        nameDrafts[v.name] ??= '';
        emailDrafts[v.name] ??= '';
        resticDrafts[v.name] ??= v.restic_repo ?? '';
      }
      await loadAuth();
      await loadLatest();
      // Cheap and local (one `git log -1` per vault), so unlike `loadLatest` it is not conditional
      // on anything and cannot fail the panel: an empty answer just leaves the line off.
      saves = await lastCommits().catch(() => []);
    } catch (e) {
      error = msg(e);
    }
  }

  /// **When each vault was last snapshotted** — the one fact somebody actually wants from a
  /// backup panel, and until 2026-09-04 no command exposed it at all.
  ///
  /// Asked here rather than folded into `backup_status`, which polls every 45 s: this is a restic
  /// spawn per vault and, for a repository that is not on this machine, a network round trip.
  ///
  /// Failures are swallowed into the answer's own `unavailable`, and a rejected call leaves the
  /// entry absent — the line simply does not render. A panel that cannot answer must not invent
  /// one, and every other thing on this screen still works.
  async function loadLatest() {
    for (const v of vaults) {
      if (!v.restic_repo) continue;
      latest[v.name] = await backupLatest(v.name).catch(() => null);
    }
  }

  /// Restic's stamp, rendered for a person. Deliberately tolerant: an unparseable value is shown
  /// as it came rather than as `Invalid Date`, because the raw string is at least a fact.
  function when(iso: string): string {
    const d = new Date(iso);
    return Number.isNaN(d.getTime()) ? iso : d.toLocaleString();
  }

  /// Git's stamp, rendered for a person: the moment, and how long ago it was.
  ///
  /// **Both halves, because they answer different questions.** The date says *what* the last save
  /// was — you recognise the afternoon you wrote it — and the elapsed count says whether anything
  /// is wrong, which is the one nobody could ask before: a vault that had not saved in thirty-nine
  /// days displayed a date like any other, and a date on its own does not subtract itself.
  ///
  /// Days, not hours: the threshold this panel's chip fires on is measured in weeks, and "4 hours
  /// ago" is precision about a question nobody has.
  function saved(at: number): string {
    const days = Math.floor((Date.now() - at * 1000) / 86_400_000);
    const ago = days <= 0 ? 'today' : days === 1 ? 'yesterday' : `${days} days ago`;
    return `${new Date(at * 1000).toLocaleString()} — ${ago}`;
  }

  /// Where credentials would come from for each HTTPS remote. Failures are left as `null`,
  /// which renders as no token row at all — a panel that cannot answer the question must not
  /// invent an answer, and everything else here still works.
  async function loadAuth() {
    for (const v of vaults) {
      if (!usesToken(v)) continue;
      auth[v.name] = await gitAuth(v.remote ?? '').catch(() => null);
    }
  }

  async function saveToken(v: PanelVault) {
    const token = tokenDrafts[v.name]?.trim();
    if (!token || busy) return;
    error = null;
    try {
      auth[v.name] = await setGitCredential(v.remote ?? '', token);
      tokenDrafts[v.name] = ''; // never keep it in the page once it is stored
      authSaved[v.name] = true;
    } catch (e) {
      error = msg(e);
    }
  }

  async function saveRestic(v: PanelVault) {
    if (busy) return;
    error = null;
    try {
      status = await setResticRepo(resticDrafts[v.name]?.trim() ?? '', v.name);
    } catch (e) {
      error = msg(e);
    }
  }

  async function savePassword() {
    const password = passwordDraft.trim();
    if (!password || busy) return;
    error = null;
    try {
      status = await setResticPassword(password);
      passwordDraft = ''; // as with the token: no secret stays in the page
      passwordSaved = true;
    } catch (e) {
      error = msg(e);
    }
  }

  async function saveRemote(v: PanelVault) {
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
</script>

<div class="panel" role="group" aria-label="back up">
  {#if noGit}
    <!-- Say what is actually wrong. Every vault below reports no remote and no identity,
           which reads as "unconfigured" — but nothing here can work until git exists, and
           inviting someone to type a remote into it would be a lie. The notebook itself is
           unaffected, and that is worth saying in the same breath. -->
    <p class="error">
      <strong>git isn't installed on this machine.</strong> Your notes are safe — they're Markdown files
      on disk and the app works normally — but nothing on this panel can run without git: no history,
      no backup, no sharing. Install git and reopen.
    </p>
  {/if}

  {#if !noRestic && status && !status.restic_password_set && vaults.some((v) => !!v.restic_repo)}
    <!-- **Per machine, so asked once** — one password unlocks every repository here. Shown only
           once a vault actually has a repo to unlock: asking for a password before there is
           anything it opens is a form with no purpose, and the repo field below is where this
           journey starts. -->
    <div class="vault">
      <div class="identity">
        <p class="why">
          Your snapshots are encrypted, and they need a password. Choose one now — it is kept in a
          file on this machine only, readable by nobody else, and never written into the vault list.
        </p>
        <div class="row">
          <input
            type="password"
            bind:value={passwordDraft}
            onkeydown={(e) => e.key === 'Enter' && void savePassword()}
            placeholder="a password you can find again"
            autocomplete="off"
            spellcheck="false"
            disabled={busy}
          />
          <button onclick={() => void savePassword()} disabled={busy || !passwordDraft.trim()}>
            Save password
          </button>
        </div>
        <!-- The one warning that is not optional. Restic cannot open a repository whose password
               is gone — there is no reset and nobody to ask. Said here because this is the only
               moment it is actionable. -->
        <p class="why">
          <strong>Write it down somewhere safe.</strong> If this password is lost, the backups it protects
          cannot be opened again — not by us, not by anyone. Your notes themselves are unaffected: they
          are plain files, and they travel with git.
        </p>
      </div>
    </div>
  {:else if passwordSaved && !noRestic}
    <div class="vault"><p class="why">✓ Backup password saved.</p></div>
  {/if}

  <!-- One block per vault: each is its own repo, its own remote, its own audience.
         A single-vault install is a list of one and reads exactly as it always did. -->
  {#each vaults as v (v.name)}
    <div class="vault">
      <!-- The repository name, matching the badge and the filter chips. `v.name` remains the
             identity every command here takes. -->
      {#if vaultRows}{@render vaultRows(v.name)}{:else if plural}<h3>{labelFor(v.name)}</h3>{/if}

      <label class="remote">
        <span
          >{plural
            ? `Where the ${v.name} vault's notes are copied to`
            : 'Where your notes are copied to'}</span
        >
        <div class="row">
          <input
            bind:value={remoteDrafts[v.name]}
            onkeydown={(e) => e.key === 'Enter' && canSaveRemote(v) && saveRemote(v)}
            placeholder="git@github.com:you/notes.git"
            spellcheck="false"
            disabled={busy}
          />
          <!-- Two "Save" buttons sit in every vault block now (a git remote and a backup repo),
                 and in a multi-vault panel that is a screenful of identically-named controls. The
                 accessible name says which one this is; the visible label stays "Save", and
                 contains it, so the two never disagree. -->
          <button
            onclick={() => saveRemote(v)}
            disabled={!canSaveRemote(v)}
            aria-label={`Save where the ${v.name} vault's notes are copied to`}>Save</button
          >
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
            Every change you send is signed with a name. Yours isn't set{plural
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

      <!-- `|| authSaved` so the confirmation is actually seen: storing the token flips
             `have_credential`, which makes `needsToken` false and would otherwise take the whole
             block — including the ✓ — off screen in the same frame. -->
      {#if needsToken(v) || authRejected(v) || authSaved[v.name]}
        <!-- The credential, asked for where it is actually needed. The clone form has had this
               field all along; a vault created *here* and later pointed at a private HTTPS repo
               had no way to reach it, so Back up failed at the push with nothing to do about it.
               Same shape as the identity block above: shown only while it is unanswered, and
               gone the moment the machine has what it needs. -->
        <div class="identity">
          {#if authSaved[v.name]}
            <p class="why">✓ Saved. Back up will use it from now on.</p>
          {:else}
            <!-- Only the wording varies. The input below is the whole point of the block, and
                   putting the rejected case in a branch of its own once left it explaining the
                   problem with no box to fix it in — the same shape as the bug being fixed. -->
            <p class="why">
              {#if authRejected(v) && !needsToken(v)}
                The token stored for this repo was refused. Paste a new one to replace it — an
                access token can expire, be revoked, or have been created without the
                <code>repo</code> scope this needs.
              {:else if auth[v.name]?.storage === 'app'}
                This repo is reached over HTTPS, which needs an access token. There is no
                system-wide git configuration on this device, so formicaria keeps it in its own
                private storage.
              {:else}
                This repo is reached over HTTPS, which needs an access token. It goes to git's own
                credential helper — the terminal and every other tool get it too, and formicaria
                keeps nothing.
              {/if}
            </p>
            <div class="row">
              <input
                type="password"
                bind:value={tokenDrafts[v.name]}
                onkeydown={(e) => e.key === 'Enter' && void saveToken(v)}
                placeholder="github_pat_…"
                autocomplete="off"
                spellcheck="false"
                autocapitalize="off"
                disabled={busy}
              />
              <button
                onclick={() => void saveToken(v)}
                disabled={busy || !tokenDrafts[v.name]?.trim()}
              >
                Save token
              </button>
            </div>
            <!-- The advice that actually limits a leak, in the same words the clone form uses.
                   A token is a *bearer* credential: it is not tied to a device, so scope and
                   expiry are the only things that bound the damage. -->
            <p class="why">
              Use a <strong>fine-grained</strong> token limited to this one repository, with contents
              read/write and an expiry date. Anyone who has the token can use it from anywhere, so a narrow
              one is the protection.
            </p>
            {#if auth[v.name]?.helper?.plaintext}
              <p class="why">
                Heads up: git on this machine uses the <code
                  >{auth[v.name]?.helper?.configured}</code
                >
                helper, which keeps credentials as <strong>plain text</strong> on disk. That is where
                this token will go.
              </p>
            {/if}
          {/if}
        </div>
      {/if}

      {#if !noRestic}
        <!-- Where this vault's *snapshot* goes. Separate field from the git remote because they
               are separate destinations with separate reasons: git carries text and keeps history,
               restic carries the whole of this vault's data and keeps none. Hidden entirely where
               restic is not installed — the capability line below already says why, and a field
               that configures a tool the machine does not have is a form that cannot be
               completed. -->
        <label class="remote">
          <span>{plural ? `The ${v.name} vault's` : "Your vault's"} backup repo</span>
          <div class="row">
            <input
              bind:value={resticDrafts[v.name]}
              onkeydown={(e) => e.key === 'Enter' && void saveRestic(v)}
              placeholder="/backup/{v.name}  ·  sftp:you@host:/backup  ·  s3:…"
              spellcheck="false"
              disabled={busy}
            />
            <button
              onclick={() => void saveRestic(v)}
              disabled={busy || (resticDrafts[v.name] ?? '') === (v.restic_repo ?? '')}
              aria-label={`Save the ${v.name} backup repo`}
            >
              Save
            </button>
          </div>
          <!-- Say what clearing does, since an empty field is how you say "nowhere". -->
          <small class="why">
            {#if v.restic_repo}
              This vault's notes and attachments are snapshotted here, encrypted — not the other
              files at the vault root, and not its git history. Clear the field and save to stop —
              nothing already backed up is removed.
            {:else}
              Empty means this vault's attachments stay on this machine. Notes are unaffected: they
              travel with git.
            {/if}
          </small>
          <!-- **"When did this last work?"** — placed right under the repository it is about,
                 because that is where somebody is deciding whether to trust it. Three states, kept
                 apart on purpose: a real snapshot, a repository that opened and has never been
                 written to, and a machine that cannot tell you. The middle one is the one that
                 should worry a reader, so it says so plainly rather than going quiet. -->
          {#if v.restic_repo && latest[v.name]}
            {@const l = latest[v.name]}
            <small class="why last-backup">
              {#if l?.unavailable}
                Last snapshot: unknown — {l.unavailable}.
              {:else if l?.time}
                Last snapshot: <strong>{when(l.time)}</strong>.
                {#if l.paths.length}
                  It covered {l.paths.length} path{l.paths.length === 1 ? '' : 's'}.
                {/if}
              {:else}
                <strong>Never backed up.</strong> This repository is configured and has no formicaria
                snapshot in it yet — pressing Back up below is what changes that.
              {/if}
            </small>
          {/if}
        </label>
      {/if}

      <ul class="promise">
        <li>
          {#if reachOf(v.remote) === 'unset'}
            <strong>Nowhere to send yet</strong> — these notes cannot leave this device.
          {:else}
            Notes → <strong>{shortDest(v.remote ?? '')}</strong> — {leaves(reachOf(v.remote))}.
            <span class="muted">Carries {carries(v)}.</span>
            <!-- **`0` and `null` are different facts and used to render identically.**
                     `{#if v.unpushed}` is falsy for zero, so "everything is pushed" and "this has
                     never been pushed" both rendered as nothing at all — the panel simply went
                     quiet, which reads as the reassuring one. They are the opposite states.
                     `null` means git could not say (no remote, never pushed, or offline — a
                     sleeping laptop is not an error), so it stays silent deliberately; zero is a
                     positive answer and now says so. -->
            {#if v.unpushed === 0}
              <span class="muted">Everything here has been sent.</span>
            {:else if v.unpushed}
              <span class="muted"
                >{v.unpushed} change{v.unpushed === 1 ? '' : 's'} not sent yet.</span
              >
            {/if}
            {#if v.identity}
              <span class="muted">Signed as {v.identity.name} &lt;{v.identity.email}&gt;.</span>
            {/if}
            <!-- **How much this is about to send.** The attachment rule selects every file at
                   or under the limit, not a diff against what has already gone — so a device that
                   has been collecting photos and never sent one has its whole backlog waiting the
                   first time it can send them. A phone has no other way to find that out: its logs
                   are unreadable, and a push that dies part-way says only that it died. Stated as a
                   size because that is the number that explains a failure. -->
            {#if v.assets_pending_bytes > OFFER_OVER}
              <span class="muted"
                >That is more than one go over a phone connection reliably finishes, so it will be <strong
                  >sent in steps</strong
                > — slower, and each step that finishes is kept, so stopping part-way loses nothing.</span
              >
            {/if}
            {#if v.assets_pending > 0}
              <span class="muted"
                >Carrying {v.assets_pending} attachment{v.assets_pending === 1 ? '' : 's'} —
                {humanSize(v.assets_pending_bytes)} in all. A large first batch over a phone connection
                can drop part-way; raising <strong>Send attachments under</strong> a step at a time in
                Settings sends them in smaller groups.</span
              >
            {/if}
          {/if}
        </li>
        <!-- **When this vault last saved anything** — stated for every vault, remote or not,
                 because it is the one fact that is true independently of everything above it. A
                 vault whose commits are blocked has a remote, an identity and a plausible unpushed
                 count; what it does not have is a recent save, and until now no screen said so.
                 Its own `<li>`, outside the remote branch, for exactly that reason: it must still
                 be there when there is no remote to talk about. -->
        <li>
          {#if savedAt(v.name) === null}
            <strong>Nothing saved here yet</strong> — this vault has no history at all.
          {:else}
            Last saved <strong>{saved(savedAt(v.name)!)}</strong>.
          {/if}
        </li>
      </ul>

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
            Remove “{labelFor(v.name)}” from this device's list? Its files stay where they are.
          </span>
          <button type="button" onclick={() => doForget(v.name)} disabled={busy}>
            Yes, remove it
          </button>
          <button type="button" onclick={() => (confirmForget = null)} disabled={busy}>
            Cancel
          </button>
        {:else}
          <button
            type="button"
            class="quiet"
            onclick={() => (confirmForget = v.name)}
            disabled={busy}
          >
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
      {:else if syncFor(v.name).phase === 'failed' && syncFor(v.name).error}
        <!-- **Read out of the sync store, not out of `steps`.** `steps` is this panel's report
               of the run you just watched; it is empty on every fresh open, so a failure from a
               previous run — or from the auto-commit loop, or from the toolbar's get-changes chip —
               would leave this screen blank with the reason sitting unread. Reported from the phone,
               2026-09-08: three saves that had not left the device, a vault named as needing
               attention, and no way to find out why. -->
        <p class="error">
          Notes did not go: {plainError(syncFor(v.name).error ?? '')}
        </p>
      {:else if v.remote_moved}
        <p class="moved">Someone has sent work you don't have yet.</p>
      {/if}
    </div>
  {/each}

  {#each unopened as u (u.name)}
    <div class="vault">
      <h3>{labelFor(u.name)}</h3>
      <p class="error">
        This vault could not be opened, so its notes are not showing: its folder is missing or
        cannot be read — <code>{u.path}</code>. If you deleted or moved it, remove it from the list;
        if it is back where it was, it opens again next time.
      </p>
      <div class="forget">
        {#if confirmForget === u.name}
          <span class="muted">
            Remove “{labelFor(u.name)}” from this device's list? Nothing is deleted.
          </span>
          <button type="button" onclick={() => doForget(u.name)} disabled={busy}>
            Yes, remove it
          </button>
          <button type="button" onclick={() => (confirmForget = null)} disabled={busy}>
            Cancel
          </button>
        {:else}
          <button
            type="button"
            class="quiet"
            onclick={() => (confirmForget = u.name)}
            disabled={busy}
          >
            Remove this vault from the list…
          </button>
        {/if}
      </div>
    </div>
  {/each}

  {#if recoveredNote}
    <p class="forget-note">{recoveredNote}</p>
  {/if}

  {#if recoverable.length}
    <!-- Deliberately right after the vaults, next to the button that removes them, and worded as
           what it is: folders already on this device, not a remote to fetch from. The note count is
           the field that answers the question actually being asked — never "which folder" but
           "which one has my work in it", since two folder names tell you nothing. -->
    <section class="recoverable">
      <h3>On this device, not in your list</h3>
      <p class="muted small">
        Removing a vault never deletes anything, so anything taken off the list by mistake is here.
      </p>
      {#each recoverable as r (r.path)}
        <div class="line">
          <strong>{r.name}</strong>
          <span class="muted">{r.notes} note{r.notes === 1 ? '' : 's'}</span>
          <button type="button" onclick={() => void takeBack(r)} disabled={busy}>
            Add it back
          </button>
        </div>
      {/each}
    </section>
  {/if}

  <!-- **What a backup carries, said here; doing it is the Back up menu's** (`decisions.md#ui`,
       2026-10-15). This used to end in a snapshot tick box and a Back up button — an action inside
       Settings, and a second way to do what the menu does. -->
  <ul class="promise">
    <li>
      <!-- Per vault, because what git carries is per vault. A flat "media is not
                 included" was wrong for any vault with an attachment limit — and wrong in the
                 dangerous direction, telling someone their attachments stayed home while they
                 were being pushed into permanent shared history. -->
      {#each vaults as v (v.name)}
        <div>
          {#if effectiveMax(v)}
            Attachments{of(v)} over {humanSize(effectiveMax(v) ?? 0)} are
            <strong>not included</strong> — smaller ones travel with the notes.
          {:else}
            Attachments{of(v)} in <code>blobs/</code> (images, PDFs, video) are
            <strong>not included</strong>.
          {/if}
        </div>
      {/each}
      {#each vaults as v (v.name)}
        {#if v.restic_repo}
          {#if v.restic_ready}
            <div>
              Snapshot{of(v)} (<em>Take a snapshot</em> in Back up) — this vault's notes and
              attachments → <strong>{shortDest(v.restic_repo ?? '')}</strong> —
              {leaves(reachOf(v.restic_repo))}.
            </div>
          {:else}
            <div>Snapshot{of(v)} <strong>will not run</strong> — {noPassword}.</div>
          {/if}
        {/if}
      {/each}
    </li>
  </ul>

  {#if error}<p class="error">{error}</p>{/if}
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

  /* A section of Settings now, not a window: it flows in Settings' own scroll surface, which
     already owns the viewport arithmetic this used to restate. */
  .panel {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    min-width: 0;
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
  .promise li {
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    background: var(--accent-subtle);
    line-height: 1.5;
  }
  .muted {
    color: var(--text-muted);
    font-weight: 400;
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
  button {
    padding: var(--space-2) var(--space-4);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    font-size: var(--text-sm);
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  /* **A narrow layout, which this panel also never had** — one `@media`, and it was
     `(pointer: coarse)`. At 390×844 the remote field and its Save button share a row, and the
     field is left showing `https://github.com/exampl` with the repository name off the end: you
     cannot read the URL you are editing.

     Last in the sheet on purpose. The base rule is `input[type='text'], .row input`, the same
     specificity as the override — placed any earlier this block loses, which is exactly what the
     first attempt did.

     Same breakpoint as `App.svelte`'s and `SettingsPanel`'s. */
  @media (max-width: 40rem) {
    /* The field takes the row; Save takes the next one. Nothing here is narrow enough to hold
       both and still show a URL. */
    .row {
      flex-wrap: wrap;
    }
    .row input {
      flex: 1 1 100%;
    }
    /* Each vault becomes a block whose edges you can see. With a single vault the `<h3>` is not
       rendered at all (`{#if plural}`), so without this the remote, the identity, the snapshot
       repo and "Remove this vault" are a stack of unattached fragments — and "which line goes
       with which option" is the question that started this work. */
    .vault {
      padding-left: var(--space-3);
      border-left: 2px solid var(--border);
    }
  }
</style>
