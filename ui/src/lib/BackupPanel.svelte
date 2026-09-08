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
  import { onMount } from 'svelte';
  import {
    backup,
    backupStatus,
    lastCommits,
    backupLatest,
    commit,
    forgetVault,
    recoverableVaults,
    createVault,
    gitAuth,
    pull,
    setGitCredential,
    setGitRemote,
    setResticPassword,
    setResticRepo,
  } from './ipc';
  import { syncVault, syncFor } from './sync.svelte';
  import { conflictLabels } from './conflictLabel';
  import { reachOf, shortDest } from './destination';
  import { GIT_ASSETS_CEILING, humanSize } from './size';
  import type { VaultInfo, Recoverable } from './types';

  /// A vault as this panel needs it: the network facts from `backup_status`, and the cheap ones
  /// from the vault list. Both used to arrive on `VaultStatus`, which meant the slow command
  /// re-described a vault it had no special knowledge of.
  type PanelVault = VaultStatus & Pick<VaultInfo, 'identity' | 'git_assets_max' | 'restic_repo'>;
  import { labelFor, refreshVaults, vaultList } from './vaults.svelte';
  import type {
    BackupRun,
    BackupStatus,
    GitAuth,
    LastCommit,
    LatestBackup,
    VaultStatus,
  } from './types';

  // `onnewvault` because this panel is already "a list, not a form" — the one surface in
  // the app that is *about the set of vaults*, which makes it where you add one. (The
  // sidebar is search-first with New note / New board; those are note gestures, and a
  // vault is not a note.)
  /// `onvaults` fires whenever this panel changes **which vaults exist** — a removal or a recovery.
  /// Without it nothing refetched `list_vaults`, so after a removal the vault filter, the "Create
  /// in" picker and the label registry all still described the world before it, until the app was
  /// relaunched. On a phone that is a long time, and it is what turned one mistaken removal into
  /// "my notes are gone" (2026-09-08).
  let {
    onclose,
    onnewvault,
    onvaults,
  }: { onclose: () => void; onnewvault: () => void; onvaults?: () => void } = $props();

  type Step = { text: string; ok: boolean };

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
  let heavy = $state(false);
  let busy = $state(false);
  let steps = $state<Step[]>([]);
  let verdict = $state<string | null>(null);
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
  // Tickable if *anyone* can take media. Vaults without a restic repo are not a reason to
  // grey out the ones that have one — they are a reason to say their media stayed put.
  const anyRestic = $derived(vaults.some((v) => v.restic_ready));
  // "restic isn't installed" and "you haven't given this vault a repo" are different
  // problems with different fixes, so never say one when you mean the other.
  const noRestic = $derived(!!status && !status.restic);
  // Something to push somewhere. A vault with no remote isn't a failure, it just has
  // nowhere to go yet.
  const anyRemote = $derived(vaults.some((v) => !!v.remote));
  /// **Either tier is enough to press the button** (2026-09-04). This used to require a git
  /// remote, so a machine set up for snapshots alone — restic installed, a repo, a password —
  /// had a tick box that ticked and a Back up button that never enabled. The panel was made to
  /// *say* so, which was honesty rather than a fix; this is the fix.
  ///
  /// The two tiers were already independent inside `run()`: a vault with no remote gets its own
  /// step line and the media loop never depended on the git loop. Only the gate assumed one.
  const canRun = $derived(!busy && ((!noGit && anyRemote) || (heavy && anyRestic)));
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
  const leaves = (r: string) => (r === 'remote' ? 'leaves this machine' : 'stays on this machine');
  const left = (r: string) => (r === 'remote' ? 'off this machine' : 'still on this machine');
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
  // **What a snapshot actually held**, in place of the fixed phrase this panel printed for as
  // long as `backup` answered nothing at all: "notes and attachments", over every vault —
  // including the ordinary one that has no attachments yet. `outstanding.md` §2.10's last
  // residue, and the level below "last backed up at": that one says *when*, this says *what*.
  //
  // `contents: null` is restic having written the snapshot and not described it. The line then
  // names what went in and stops, because printing zeros a reader cannot tell from an empty
  // vault is the overstatement in the other direction.
  const held = (r: BackupRun) => {
    const dirs: string[] = [];
    if (r.notes_dir) dirs.push(`${r.notes_dir}/`);
    if (r.blobs) dirs.push('blobs/');
    // Both absent is refused by `backup` before it runs — there would be nothing to snapshot —
    // so this fallback should never reach a screen. It is here because a blank where a
    // directory name belongs reads as a bug in the panel rather than in the answer.
    const what = dirs.join(' + ') || 'nothing this vault owns';
    if (!r.contents) return `${what}, contents not reported`;
    const c = r.contents;
    const files = c.files_new + c.files_changed + c.files_unmodified;
    const fresh = c.files_new + c.files_changed;
    // "Nothing changed" is worth saying plainly: a snapshot that added no bytes is the healthy
    // steady state, not a failure, and a panel that only ever reports numbers makes it look like
    // one.
    return fresh
      ? `${what}, ${files} file(s), ${fresh} new or changed, ${humanSize(c.bytes_added)} added`
      : `${what}, ${files} file(s), nothing changed since the last one`;
  };
  // The password is the app's now, and the environment variable is the override rather than the
  // mechanism. Naming `RESTIC_PASSWORD` as *the* thing that is missing sent people to a launcher
  // script to fix something the field above this line fixes.
  const noPassword = 'no backup password is set on this machine';
  // A single vault has no boundary to talk about, so don't name it at every turn.
  const plural = $derived(vaults.length > 1);
  const of = (v: PanelVault) => (plural ? ` (${v.name})` : '');

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

  // Bring their work home. Separate from "Back up" on purpose: pushing and pulling are
  // different intentions, and a button that quietly did both would be a button nobody
  // could predict. Commit first for the same reason `run()` does — the 5s auto-commit
  // is best-effort, and git will not merge over uncommitted edits.
  async function bringDown(v: PanelVault) {
    if (busy) return;
    busy = true;
    steps = [];
    verdict = null;
    error = null;
    try {
      // Commit first (git will not merge over uncommitted edits) — but an unfinished merge must
      // stop us here rather than fall into a pull that will only refuse, with git's wording
      // instead of ours.
      //
      // **Keyed on the conflicts, not on `committed`.** Until 2026-09-07 a mid-merge commit
      // committed nothing, so `!committed` was a reliable proxy for "mid-merge". It is not any
      // more: `commit_all` now commits every path except the conflicted ones, so the ordinary
      // outcome here is `committed: true` **with** notes still stuck — and the old test would
      // have sailed past into a pull that refuses. What blocks a *sync* is the unfinished merge
      // itself, which is exactly what `conflicts` reports.
      const c = await commit(`auto: ${new Date().toISOString()}`, v.name).catch(() => null);
      const blocked = !!c && c.conflicts.length > 0;
      if (blocked) {
        // Not an early return: `busy = false` lives after this block, not in a `finally`,
        // so returning here would leave the panel frozen.
        steps.push({
          text: `${c!.conflicts.length} note${c!.conflicts.length === 1 ? '' : 's'} in ${v.name} still need you: ${(await conflictLabels(c!.conflicts)).join(', ')}. Everything else here is being saved to history as usual, but this vault cannot sync with the other device until ${c!.conflicts.length === 1 ? 'it is' : 'they are'} settled — see “Needs resolution” in the Collaboration view, which says what to do for each one.`,
          ok: false,
        });
      }
      const r = blocked ? { merged: 0, conflicts: [], kept: [] } : await pull(v.name);
      if (blocked) {
        // nothing further to report; the line above is the answer
      } else if (r.conflicts.length) {
        steps.push({
          text: `Merged${of(v)}, but ${r.conflicts.length} note${r.conflicts.length === 1 ? '' : 's'} need you: ${(await conflictLabels(r.conflicts)).join(', ')}. See “Needs resolution” in the Collaboration view: some are resolved by editing the note, and some only by choosing a side.`,
          ok: false,
        });
      } else if (r.merged) {
        steps.push({
          text: `Brought in ${r.merged} change${r.merged === 1 ? '' : 's'}${of(v)} — combined cleanly.`,
          ok: true,
        });
      } else {
        steps.push({ text: `Already up to date${of(v)}.`, ok: true });
      }
      // **Said whichever way the pull went**, and outside the else-if chain on purpose: a note the
      // app brought back is a decision it made for you, and it must not be the thing that gets
      // dropped because some other branch reported first.
      if (r.kept.length) {
        steps.push({
          text:
            `Kept ${r.kept.length} note${r.kept.length === 1 ? '' : 's'}${of(v)} the other device had deleted: ` +
            `${(await conflictLabels(r.kept)).join(', ')}. There was no text to merge, and leaving it ` +
            `unsettled would have stopped this vault committing anything at all. If the deletion was ` +
            `deliberate, delete ${r.kept.length === 1 ? 'it' : 'them'} again here.`,
          ok: true,
        });
      }
    } catch (e) {
      steps.push({ text: `Could not get their changes${of(v)}: ${msg(e)}`, ok: false });
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
    for (const v of vaults) {
      if (!v.remote) {
        stuck.push(v.name);
        steps.push({
          text: `Nowhere to send${of(v)} yet — those notes cannot leave this device.`,
          ok: false,
        });
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
          text: `Notes${of(v)} sent to ${shortDest(v.remote)} — ${left(reach)}.`,
          ok: true,
        });
        (reach === 'remote' ? off : stuck).push(v.name);
      } else if (phase === 'conflicts') {
        // Deliberately not pushed. Those notes hold both versions in their bodies, and
        // publishing conflict markers as content is worse than not publishing.
        steps.push({
          text:
            `Notes${of(v)} NOT sent — ${s.conflicts.length} note(s) came back with ` +
            `conflicting edits and need you first: ${(await conflictLabels(s.conflicts)).join(', ')}`,
          ok: false,
        });
        stuck.push(v.name);
      } else {
        steps.push({ text: `Notes${of(v)} NOT sent: ${msg(s.error)}`, ok: false });
        stuck.push(v.name);
      }
    }

    // The media tier, per vault and for the same reason git is: a restic repo is per
    // repository, so each vault either has one or its media has nowhere to go. Back up
    // the ones that can and **name the ones that can't** — silently skipping them is the
    // failure this panel exists to prevent. Independent of the git tier: a failed push
    // must not cancel a snapshot.
    if (heavy) {
      for (const v of vaults) {
        if (!v.restic_ready) {
          noMedia.push(v.name);
          steps.push({
            text: v.restic_repo
              ? `Snapshot${of(v)} NOT taken — ${noPassword}.`
              : `Snapshot${of(v)} NOT taken — no backup repo configured for it.`,
            ok: false,
          });
          continue;
        }
        const reach = reachOf(v.restic_repo);
        try {
          const run = await backup(v.name);
          steps.push({
            text: `Snapshot${of(v)} — ${held(run)} → ${shortDest(v.restic_repo ?? '')} — ${left(reach)}.`,
            ok: true,
          });
          if (reach === 'remote') mediaOff.push(v.name);
        } catch (e) {
          steps.push({ text: `Snapshot${of(v)} failed: ${msg(e)}`, ok: false });
          noMedia.push(v.name);
        }
      }
    }

    // Name the vaults that did not make it, on both tiers. "Your notes are backed up"
    // while one vault sat still is the one sentence this panel must never say. And the
    // failures are not all one cause — a vault reaches `noMedia` from a missing repo, a
    // missing password *or* a restic error, so the verdict names the outcome and lets the
    // step above it give the reason.
    // **A tier that never ran has no verdict.** With no remote anywhere, every vault lands in
    // `stuck` and the git sentence would report a failure where there was no attempt — and
    // "your notes are still on this machine" is flatly wrong when the snapshot tier just sent
    // them somewhere, because a snapshot carries the notes directory as well as `blobs/`.
    const gitRan = !noGit && vaults.some((v) => !!v.remote);
    verdict =
      (!gitRan
        ? 'No vault has anywhere to send to, so nothing was sent.'
        : stuck.length === 0
          ? 'Your notes are off this machine.'
          : off.length === 0
            ? 'Your notes are still on this machine.'
            : `Notes off this machine: ${off.join(', ')}. Still here: ${stuck.join(', ')}.`) +
      ' ' +
      (!heavy
        ? 'No snapshot was taken.'
        : noMedia.length === 0
          ? mediaOff.length
            ? 'Your snapshot is off this machine.'
            : 'Your snapshot was taken, but is still on this machine.'
          : mediaOff.length === 0
            ? `No snapshot was taken (${noMedia.join(', ')}).`
            : `Snapshot off this machine: ${mediaOff.join(', ')}. Not taken: ${noMedia.join(', ')}.`);
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
            file on this machine only, readable by nobody else, and never written into the vault
            list.
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
        {#if plural}<h3>{labelFor(v.name)}</h3>{/if}

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
                read/write and an expiry date. Anyone who has the token can use it from anywhere, so a
                narrow one is the protection.
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
                Empty means this vault's attachments stay on this machine. Notes are unaffected:
                they travel with git.
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

        {#if steps.length === 0}
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
          <!-- **The detail the toolbar promised.** "…need you: open backup options for detail"
               pointed here, and here had nothing to show: `steps` is only ever filled by this
               panel's own buttons, so a backup started from the toolbar left this screen blank and
               the reason sitting unread in the sync store. Reported from the phone, 2026-09-08 —
               three unpushed commits, a vault named as needing attention, and no way to find out
               why. Persistent, because the message that sends you here can arrive at any time. -->
          <p class="error">
            Notes did not go: {syncFor(v.name).error}
          </p>
        {:else if v.remote_moved}
          <p class="moved">Someone has sent work you don't have yet.</p>
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
          Removing a vault never deletes anything, so anything taken off the list by mistake is
          here.
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

    {#if steps.length === 0}
      <ul class="promise">
        <li>
          {#if !heavy}
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
          {:else}
            {#each vaults as v (v.name)}
              {#if v.restic_ready}
                <div>
                  Snapshot{of(v)} — this vault's notes and attachments →
                  <strong>{shortDest(v.restic_repo ?? '')}</strong> —
                  {leaves(reachOf(v.restic_repo))}.
                </div>
              {:else}
                <div>
                  Snapshot{of(v)} <strong>will not run</strong> —
                  {v.restic_repo ? noPassword : 'no backup repo for it'}.
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
        <!-- **"Media" was never what this tier does.** The snapshot is the notes directory
             *and* `blobs/`, which is why a restore returns a working vault and not a pile of
             images — and why calling it "media" left people believing their notes were in git
             alone. It is still opt-in and still the heavy half; it is just not media-only. -->
        Include an encrypted snapshot — notes and attachments, via restic
        {#if noRestic}
          <span class="muted">
            (unavailable: restic isn't installed on this machine — the snapshot is an optional
            feature, and your notes don't need it)
          </span>
        {:else if status && !anyRestic}
          <!-- Both halves of what this used to say were stale: it sent people to edit the vault
               list for a repo the field above sets, and to restart for a password that takes
               effect the moment it is saved. Name the field, not the file. -->
          <span class="muted">
            (unavailable: {vaults.some((v) => !!v.restic_repo)
              ? 'set a backup password above'
              : 'give a vault a backup repo above, and set a password'})
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

    <!-- **A disabled button has to say why it is disabled.** `canRun` needs a git remote, so a
         machine set up for snapshots alone — restic installed, a repo, a password — offered a
         tick box that ticked and a Back up button that never enabled, with nothing on screen
         explaining it. Saying so is not the fix; running the snapshot tier on its own is, and
         that is queued rather than smuggled in here. Until then the panel is at least honest
         about its own refusal. -->
    {#if status && !canRun && !busy}
      <p class="why">
        {#if !noGit && !anyRemote && anyRestic && !heavy}
          No vault has a git remote yet, so there is nothing to push — but this machine can take a
          snapshot. Tick the box above to run that tier on its own.
        {:else if !noGit && !anyRemote}
          Back up needs a git remote to send notes to, and no vault has one yet — set one above.
        {:else if noGit && !anyRestic}
          This machine has neither git nor a configured snapshot repository, so there is nowhere for
          anything to go.
        {:else if noGit}
          Git is not installed, so notes cannot be pushed. Tick the box above to take a snapshot
          instead — that tier carries the notes as well as the attachments.
        {/if}
      </p>
    {/if}

    <div class="actions">
      <button onclick={onclose} disabled={busy}>Close</button>
      <button class="primary" onclick={run} disabled={!canRun}>
        {busy
          ? 'Backing up…'
          : `Back up ${plural ? 'every vault' : 'notes'}${heavy ? ' + snapshot' : ''}`}
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
    .backup-overlay {
      padding-bottom: max(var(--safe-bottom), var(--bar-floor));
    }
  }
  .backup-backdrop {
    position: fixed;
    inset: 0;
    border: none;
    background: rgb(0 0 0 / 0.45);
    cursor: default;
  }
  /* **One scroll surface, and it is this one.** With two vaults registered, or with the
     restic-password block open, this panel is taller than a phone screen — and it used to
     have no `max-height` and no `overflow` at all, so Close and the Back up button simply
     fell off the bottom with nothing to scroll them into view. Resist the temptation to
     scroll an inner body under a pinned head instead: `UnrecordedPanel` shipped that shape
     and a finger landing on the inner region moved the inner region while the panel stayed
     put, which made everything below it unreachable in a different way. */
  .panel {
    position: relative;
    width: min(34rem, 92vw);
    /* `100%` of the overlay's content box, which is already the visible viewport less the
       top offset and both window insets. */
    max-height: 100%;
    overflow-y: auto;
    /* Don't chain the drag to the shell when this hits its end. */
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
