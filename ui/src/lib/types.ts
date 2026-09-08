// Mirrors the Rust DTOs in crates/fm-app/src/dto.rs. `props` is an open map, so
// a custom frontmatter property reaches the UI with no type change.
export interface ObjectMeta {
  id: string;
  type: string;
  title: string | null;
  preview: string;
  /** Several lines of the body, **feed only** — `recent()` fills it and nothing else does. Absent
   *  on every other list, so treat it as optional at every call site. Char-capped server-side;
   *  see `dto.rs`'s `excerpt`. */
  excerpt?: string;
  status: string | null;
  due: string | null;
  start: string | null;
  hard: boolean;
  created: string;
  updated: string;
  tags: string[];
  /** Content-addressed blob refs (`sha256:<hex>`) — a gallery tile fetches its
   *  thumbnail from the first one. */
  assets: string[];
  props: Record<string, unknown>;
  /** Which vault — i.e. which audience — this note belongs to. Derived from where the
   *  file lives, never from what it says, so a badge reading "lab" is telling the truth
   *  about who can see it. Empty in a single-vault install: no boundary, no badge. */
  vault: string;
}

/** One note's most-recent edit, read from git (`crates/fm-app/src/commands.rs::EditEvent`):
 *  who last touched it and when. Powers the "edited by" labels, the activity stream, and the
 *  contributor filter — collaboration facts git already knows, none stored by the app. */
export interface EditEvent {
  id: string;
  title: string | null;
  type: string;
  vault: string;
  author: string;
  email: string;
  /** ISO-8601 author date. */
  time: string;
}

export interface Column {
  /** The settable string echoed back to set_property on drop; empty clears. */
  value: string;
  label: string;
  cards: ObjectMeta[];
}

export interface Board {
  group_by: string;
  columns: Column[];
}

/** A single note with its full body — the read view's payload (`get`). */
export interface NoteDetail extends ObjectMeta {
  body: string;
  /** Hash of `body` as read — send it back as `update_body`'s `base`. See the Rust doc. */
  version: string;
}

/** One message in a note's discussion.
 *
 *  A message is an ordinary note carrying `thread_of` — there is no message *kind*. `depth` is
 *  computed on the server from a flat list, so the UI never walks `reply_to` itself: that
 *  pointer is hand-editable frontmatter and can dangle or cycle, and a client-side walk would
 *  meet those as a blank pane or a hang. */
export interface ThreadMessage extends ObjectMeta {
  body: string;
  /** The message this answers, or null when it answers the note (or the pointer is unusable). */
  reply_to: string | null;
  /** Indentation level, already capped. Derived, never stored. */
  depth: number;
}

/** A note's discussion. `root` is null when the note itself has been deleted — the reasoning
 *  about a note outlives the note. */
export interface ThreadView {
  root: ObjectMeta | null;
  count: number;
  messages: ThreadMessage[];
}

/** A proposal's change, for review: the unified diff against `main` and the files it touches.
 *  `exists` is false (with an empty diff) when the branch is gone — merged or deleted — because a
 *  proposal note outlives its branch. */
export interface ProposalDiff {
  exists: boolean;
  /** Rejected: branch gone, note kept as a record — distinguishes a declined PR from a merged one. */
  declined: boolean;
  files: string[];
  patch: string;
}

/** The proposed note behind a proposal — for the review to show and edit before accepting. */
export interface ProposalContent {
  /** The note this proposal edits (an edit is saved back through `create_proposal` on this note). */
  host: string;
  title: string;
  /** The proposed note body as it stands on the branch. */
  body: string;
}

/** One first-class discussion, as the Discussions view shows it at a glance. The `ObjectMeta`
 *  fields are the discussion's own note (a self-rooted note — `thread_of` points at itself), so
 *  `title` and `vault` render directly. */
export interface DiscussionSummary extends ObjectMeta {
  /** Messages in the discussion (not counting the root note). */
  count: number;
  /** ISO-8601 of the newest message, or the discussion's own creation when empty — the sort key. */
  last_activity: string;
  /** Who has posted, newest-first, from git authorship (never stored). Empty when the vault has no
   *  history yet — the discussion still shows its title and vault. */
  participants: Identity[];
}

/** Whether a referenced asset can be shown, and its sniffed MIME. */
export interface AssetStatus {
  has_blob: boolean;
  has_thumb: boolean;
  mime: string | null;
}

/** Who a vault's commits are signed by — the name a collaborator sees in `git log`. */
export interface Identity {
  name: string;
  email: string;
}

/** One vault's git standing. Per vault, not per app: one vault is one repo, one remote,
 *  one collaborator list, so there is no honest way to collapse these into one number. */
export interface VaultStatus {
  /** The audience. Also the argument every git command takes back. */
  name: string;
  /** Where this vault's notes push to, or null when no remote is set yet. */
  remote: string | null;
  /** Commits made here but not on the remote; null when never pushed. */
  unpushed: number | null;
  /** Null when nobody real signs this vault's commits — either git has no identity
   *  configured, or it still holds the placeholder. A remote cannot be set while
   *  this is null, because git history is forever and an unattributed shared vault
   *  cannot answer "who touched this?". */
  identity: Identity | null;
  /** Someone else has pushed work we don't have. Null when unknowable: no remote,
   *  never pushed, or simply offline — a sleeping laptop is not an error. */
  remote_moved: boolean | null;
  /** Notes with conflict markers in them, waiting for a human. The `.md` merge driver
   *  keeps markers out of the frontmatter, so these still open in the editor. */
  conflicts: string[];
  /** Where this vault's media backs up to — a path or URL, never the password. Null when
   *  this vault has no restic repo, which is not an error: a restic repo is per
   *  repository, so a set of vaults needs one each. */
  restic_repo: string | null;
  /** This vault's media could actually be backed up **now**: restic is installed, this
   *  vault has a repo, and the password is set. All three — "ready" has to mean
   *  "will work", not "is configured". */
  restic_ready: boolean;
  /** The largest attachment this vault sends with its notes; null is the default, notes
   *  only. Here so the git tier can state its own scope: "media is not included" is false
   *  for any vault with a limit, whose blobs at or under it are committed and pushed. Set
   *  in Settings ("Send attachments under"), read here. */
  git_assets_max: number | null;
}

/** What each backup tier could do right now (`backup_status`).
 *
 *  **Both tiers are per vault**: git because one vault is one repo and one remote, restic
 *  because a restic repo is per repository too. There is no app-wide destination for
 *  either, which is why there is no field here for one. */
export interface BackupStatus {
  /** Every vault, in configured order; the first is the default for new notes. A
   *  single-vault install is a list of one. */
  vaults: VaultStatus[];
  /** Whether this machine has git. Without it every vault reports `remote: null,
   *  identity: null`, which reads exactly like "not set up yet" — so the panel has to be
   *  told, or it would invite you to configure a tier that cannot run. */
  git: boolean;
  /** Whether this machine has restic. Distinct from a vault's `restic_ready`: "no restic
   *  installed" and "restic installed but this vault has no repo" are different things to
   *  tell someone. */
  restic: boolean;
  /** Whether this machine holds the restic password — one for every repository here, and a bool
   *  rather than the value, which nothing on this side needs. The third of the three conditions
   *  `restic_ready` folds together, reported on its own so the panel can name which is missing. */
  restic_password_set: boolean;
}

/** When a vault's media was last snapshotted — the one fact a backup panel most needs.
 *
 *  **Its own call, not a field on `VaultStatus`.** `backup_status` is polled every 45 s and
 *  already shells out per vault; asking restic for the latest snapshot is another spawn each
 *  time, and against a network repo a round trip. So this is fetched when a human is looking. */
export interface LatestBackup {
  /** The vault this is about, echoed back so several answers can be keyed. */
  vault: string;
  /** Restic's short snapshot id, or null when the repo has no `fm`-tagged snapshot yet.
   *  **Null is not an error** — a freshly configured repo has never been written to, and
   *  "never" is the useful answer rather than a failure. */
  id: string | null;
  /** When it was taken, in restic's own words (RFC 3339). Null with `id`. */
  time: string | null;
  /** The **source** paths it recorded, absolute on whatever machine took it. A snapshot taken
   *  on another device names that device's paths, and a restore that silently used them is the
   *  failure this makes visible before it happens. */
  paths: string[];
  /** Why there is no answer, when there is none to be had — no restic, no repo, no password, or
   *  a repository that would not open. Distinguished from `id: null` on purpose: *never backed
   *  up* and *this machine cannot tell you* are different sentences to put in front of someone. */
  unavailable: string | null;
}

/** What one `backup` run put in the repository.
 *
 *  **The command used to answer nothing at all.** A snapshot could be reported as taken and
 *  nothing said about what was in it, so the panel filled the hole with a fixed phrase — "notes
 *  and attachments" — over every vault, including the ordinary one that has no attachments yet.
 *  Mirrors `fm_app::dispatch::BackupRun`. */
export interface BackupRun {
  /** The vault this is about; the panel runs the snapshot tier per vault and writes a line each. */
  vault: string;
  /** The notes directory that went in, by name — `notes`, or whatever `vault.json` calls it.
   *  Null for a vault that has none yet, which is a real state: a vault holding only `blobs/` is
   *  still worth snapshotting. */
  notes_dir: string | null;
  /** Whether `blobs/` existed and went in. */
  blobs: boolean;
  /** Restic's own account of the snapshot. **Null is "restic did not say", not "it was empty"** —
   *  the same distinction `LatestBackup` draws between `id: null` and `unavailable`. Nested
   *  rather than flattened so that one null covers the whole summary; six nullable numbers would
   *  put a reader back to guessing which zero was really a zero. */
  contents: SnapshotContents | null;
}

/** Restic's numbers for the snapshot it just wrote. Mirrors `fm_core::backup::Contents`. */
export interface SnapshotContents {
  /** The short id, named the way `LatestBackup.id` names it. */
  id: string;
  files_new: number;
  files_changed: number;
  /** Files already in the repository. On a healthy vault this is most of them — it is how much
   *  of the backup was free. */
  files_unmodified: number;
  /** Bytes read out of the vault. */
  bytes_processed: number;
  /** Bytes the repository grew by; deduplication is why it is normally a fraction of the above. */
  bytes_added: number;
}

/** A conflicted note **and what kind of conflict it is**.
 *
 *  `has_markers` is the field that matters. For a delete/modify conflict it is `false` and there is
 *  nothing in the note to edit — one side has no file, so git writes no markers and never will. The
 *  UI told every user, in every case, to "open each one, both versions are marked in the text";
 *  following that advice on a marker-less conflict is impossible, and while it sat unresolved its
 *  vault committed nothing at all. Mirrors `fm_app::dto::ConflictInfo`. */
export interface ConflictInfo {
  note: ObjectMeta;
  /** Vault-relative path git is unmerged on. Empty when this came from the body-marker scan. */
  path: string;
  vault: string;
  /** Git's two-letter code (`UU`, `DU`, `UD`, …). */
  code: string;
  /** One plain sentence: what the two sides did. */
  what: string;
  has_markers: boolean;
}

/** One family of identical notes: which copy stays, and which are extras. Mirrors
 *  `fm_app::commands::DuplicateFamily`. */
export interface DuplicateFamily {
  /** sha256 of the shared body — the family's identity, stable across pruning. */
  body: string;
  vault: string;
  /** First line of the shared body, so a human can recognise what got duplicated. */
  preview: string;
  /** The copy that stays: oldest by `created`. Never offered for deletion. */
  keep: string;
  extras: string[];
}

/** One field two devices set differently, where the merge kept both.
 *
 *  The note shows `kept`; `other` is what the other device said, still in the file as a
 *  `conflict-<field>` key. Mirrors `fm_app::commands::DemotedField`. Both values are display
 *  strings: promoting one goes through `set_property`, the same path a person typing it takes. */
export interface DemotedField {
  id: string;
  vault: string;
  title: string;
  field: string;
  kept: string;
  other: string[];
}

/** One thing a merge brought back after the other device had deleted it.
 *  Mirrors `fm_app::commands::KeptNote`.
 *
 *  `id` and `title` are `null` for a kept path that is **not** a note — the keep branch settles
 *  every delete/modify path in the repo, so a saved view or the attachment manifest can be
 *  resurrected too. Those are reported and given no button: "delete it again" is the `delete`
 *  command, and that command is about notes. */
export interface KeptNote {
  path: string;
  id: string | null;
  vault: string;
  title: string | null;
}

/** When each vault last saved anything — seconds since the epoch, `null` for a vault that has
 *  never been committed. Mirrors `fm_app::dispatch::LastCommit`.
 *
 *  **Seconds, not milliseconds**, because that is what git records; the UI multiplies. And `null`
 *  is a third state, not a zero: a vault with no history has nothing to nag about, and treating a
 *  missing value as `0` would announce fifty-six years of silence to someone on their first run. */
export interface LastCommit {
  vault: string;
  last_commit: number | null;
}

/** Notes on disk that git does not have, per vault — what the app forgot it wrote.
 *  Mirrors `fm_app::dto::Unrecorded`. */
export interface Unrecorded {
  vault: string;
  count: number;
  /** **Split by kind, because the kinds mean opposite things.** `new` notes exist nowhere else if the
   *  vault has no remote; a pile of `modified` means something is rewriting notes it did not need to;
   *  `deleted` means the *deletion* is what git has not recorded. A bare count could not tell these
   *  apart, which is why "146 not in history" on the phone was a number nobody could act on. */
  new: number;
  modified: number;
  deleted: number;
  /** A bounded sample (50) with enough detail to recognise what happened. The counts are the
   *  complete picture; this is the evidence. */
  notes: UnrecordedNote[];
}

/** One note git does not have, in enough detail to recognise it without a shell — which on a phone is
 *  the only option: no readable logcat, no console, no terminal. */
export interface UnrecordedNote {
  id: string;
  path: string;
  /** `new` | `modified` | `deleted`. */
  kind: string;
  /** Title, or first body line. `null` for a deleted note — there is no file left to read, and
   *  inventing a name would be worse than admitting that. */
  title: string | null;
  bytes: number | null;
  /** When the *file* was written. A copy, restore or migration resets this for every file at once, so
   *  it is not "when the note was made" — that is `created`. */
  modified: string | null;
  /** The note's own `created`, from its frontmatter. Survives copying. */
  created: string | null;
  /** `note` | `message` | `proposal` | `unreadable` | `deleted` — which code path wrote it. A title
   *  says what a note is about; this says who made it, which is the question when 142 appear in one
   *  minute. */
  role: string;
  /** How many outstanding notes in this vault share this exact body, this one included. `1` is
   *  normal; higher is the finding. */
  copies: number;
}

/** What a commit did. `committed: false` with conflicts listed is not "nothing to do" —
 *  it is "this vault is mid-merge, so nothing will be committed until a human settles it".
 *  Those two used to be the same `false`, which is how a conflicted note could silently
 *  stop every later save from ever being committed. */
export interface CommitResult {
  committed: boolean;
  conflicts: string[];
}

/** What a pull did. Conflicts are a result, not a failure. */
export interface PullResult {
  merged: number;
  conflicts: string[];
  /** Notes the other device had **deleted** while this one edited them, kept rather than left to
   *  freeze the vault (`decisions.md`, 2026-09-07). There is no text to merge in that case, and an
   *  unmerged index refuses every commit in the vault — which is how two notes stopped two hundred
   *  for thirty-nine days.
   *
   *  **The caller must say so.** This is a decision the app made on the user's behalf: it discards
   *  a deletion, and the undo is "keep this device's version" in Needs resolution. Empty in the
   *  ordinary case. */
  kept: string[];
}

/** A vault, as the sidebar and the first-run screen need it. Deliberately not
 *  `VaultStatus`: that one is about a remote and costs a `git ls-remote` per vault, and
 *  the first-run screen — shown when there are none — must not wait on the network. */
export interface VaultInfo {
  name: string;
  /** For display, so two vaults both called `notes` are tellable apart. */
  path: string;
  /** Index 0 — where every fresh capture lands. */
  default: boolean;
  /** Attachments up to this many bytes travel with this vault's notes; `null` means none do.
   *  Lives in the vault's own `vault.json`, not in this browser, because it decides what enters
   *  shared permanent history. */
  git_assets_max: number | null;
  /** What may be done with this vault's review record. Two independent answers, never one flag:
   *  agreeing to record something locally is not agreeing to publish it. */
  supervision: { collect: boolean; publish: boolean };
  /** **What to show instead of `name`**: the repository behind this vault's remote
   *  (`…/formicarium-vault.git` → `formicarium-vault`), or `null` for a vault with no remote — or
   *  when two vaults would derive the same label. Display only; `name` stays the identity. See
   *  `vaultLabels.svelte.ts`. */
  label: string | null;
  /** The committer this vault signs with, or `null` when git has never been told who you are.
   *
   *  **Here and not in `BackupStatus`** because the welcome screen gates on it, and that gate sits
   *  on the path that renders the app. `backup_status` runs a network `git ls-remote` per vault;
   *  a first-run screen that waits on it waits on the network. This list already spawns git
   *  locally for `label`, so the answer is one more local read beside a call already happening. */
  identity: { name: string; email: string } | null;
}

/** What would happen if we created a vault at a path. The server owns `ok`: duplicating
 *  the policy here is how you get a button that enables and then fails. */
export interface PathCheck {
  path: string;
  exists: boolean;
  empty: boolean;
  /** `.md` already under `<path>/notes`. They will be adopted — say so, never surprise. */
  notes: number;
  not_a_directory: boolean;
  parent_missing: boolean;
  writable: boolean;
  git_repo: boolean;
  name_ok: boolean;
  name_taken: boolean;
  path_taken: boolean;
  /** The vault this path nests in, or that nests in it. */
  overlaps: string | null;
  config_writable: boolean;
  ok: boolean;
}

/** A kind of file an import has no landing site for, and how many there were. */
export interface LeftBehind {
  kind: string;
  count: number;
}

/** What importing a folder would involve — **and whether it can be**. The server owns `ok`
 *  and answers for the source *and* the destination together: two verdicts ANDed in the
 *  browser is the second opinion that makes a button enable and then fail. */
export interface ImportCheck {
  /** `logseq` | `obsidian`, or null when the folder is neither. */
  format: string | null;
  label: string | null;
  pages: number;
  journals: number;
  attachments: number;
  attachmentBytes: number;
  leftBehind: LeftBehind[];
  /** One sentence, the most disqualifying first. Null means it can be imported. */
  problem: string | null;
  ok: boolean;
}

/** What an import actually did. Every number is reported, including the ones that are not
 *  good news — a dangling link or a renamed property is something the user should learn now
 *  rather than discover in a month. */
export interface ImportReport {
  format: string;
  notes: number;
  stubs: number;
  alreadyImported: number;
  attachments: number;
  deduped: number;
  links: number;
  dangling: number;
  danglingNames: string[];
  blocks: number;
  blocksUnresolved: number;
  renamedProperties: number;
  leftBehind: LeftBehind[];
  warnings: string[];
  /** Whether it all went into history as one entry — which is what makes it undoable as one. */
  recorded: boolean;
  vault: string;
}

/** The renderer a `.view` draws through — the same set the built-in nav offers. */
export type Renderer = 'board' | 'agenda' | 'timeline' | 'search' | 'gallery';

/** One saved `.view`, as the sidebar lists it. A view that would not parse still appears,
 *  with `error` set and `renderer` null — a broken view names itself, never vanishes. */
/** One `themes/*.css` file in a vault. The filename stem is the name — there is no header format
 *  inside the file to carry a prettier label, on purpose. `error` is set for a file we can see but
 *  cannot apply: a broken theme names itself rather than silently doing nothing. */
export interface ThemeInfo {
  name: string;
  /** Which vault holds it — needed to read or delete it, for the same reason as `ViewInfo.vault`. */
  vault?: string;
  bytes: number;
  error?: string;
}

export interface ViewInfo {
  name: string;
  renderer: Renderer | null;
  group_by: string | null;
  /** Which vault holds the file. Needed to delete it: without it the command resolves against the
   *  default vault, where a view belonging to another one is simply not found — and a delete that
   *  reports success while deleting nothing is the worst answer available. */
  vault?: string;
  error?: string;
}

/** The result of running a view: a `board` for the board renderer, a flat `rows` list
 *  otherwise. One envelope; the UI switches on `renderer` exactly as for built-ins. */
export interface ViewResult {
  name: string;
  renderer: Renderer;
  group_by: string | null;
  /** What this view leaves out, one phrase per `filter:` entry ("status is not done"), from the
   *  server. Empty for a view that filters nothing — and empty is *sent*, never omitted, so there
   *  is one shape to handle.
   *
   *  A `view: board` draws through the same renderer as the Board pane, so a filter that removes a
   *  column removes it invisibly; this is what lets the pane say so. It travels with the payload
   *  it describes rather than with the view *list*, which is fetched once per vault change and has
   *  failed on the phone — words that arrive late or not at all are the bug over again. */
  filters: string[];
  board?: Board;
  rows?: ObjectMeta[];
}

/** What this installation is configured as — the Settings screen's whole payload.
 *
 *  **Read-only by construction**, not by preference: `vaults::save` is append-only and never
 *  rewrites an existing entry, so offering to edit a vault's path or restic repo here would
 *  silently do nothing. Where something *is* editable it stays where it already is — the
 *  backup panel owns remotes and identity. Deliberately cheap to fetch: no shelling out, so
 *  opening Settings never triggers the per-vault `git ls-remote` that `backup_status` does. */
export interface Config {
  /** Which build this is — the release tag, or `dev` for anything built locally. Baked in at
   *  build time, because the crates carry no version of their own. A string to read: there is no
   *  update check behind it and nothing is fetched. It exists because each release unpacks into
   *  its own folder, so someone who has updated has two of them and no other way to tell which
   *  one is running. */
  version: string;
  /** The vault list file we would write; `null` when this machine has no config dir at all. */
  vault_list: string | null;
  /** False also means "unparseable, so we will never overwrite it" — not merely "no permission". */
  vault_list_writable: boolean;
  vaults: VaultInfo[];
  restic: { vault: string; repo: string | null }[];
  /** `FM_*` overrides actually in effect. Never contains a secret. */
  env: { name: string; value: string }[];
  git: boolean;
  /** Whether restic is on this machine. Three different questions used to be answerable
   *  only as one: installed (this), configured for a vault (`restic` above), and unlocked
   *  (`restic_password_set`). Conflating them is how a control enables for a tool that is
   *  not there. */
  restic_installed: boolean;
  /** Whether the text inside a PDF can be read out, so a paper is searchable by its contents. */
  pdf_text: boolean;
  /** Present/absent only. The value is never sent. */
  restic_password_set: boolean;
  /** The one directory this installation puts vaults in, or `null` when the user chooses.
   *  Present on a phone, absent on a desktop — and it is what decides whether the new-vault
   *  form asks for a folder at all. */
  vault_root: string | null;
  /** What happened when this build gave its bundled OpenSSL a CA trust store — a count, or
   *  why there is none. `null` on a desktop, which uses the system store. */
  ca_bundle: string | null;
  /** Which OS this build runs on: `linux` | `macos` | `windows` | `android` | `ios`.
   *
   *  A fact, not a policy — the backend reports the OS and this side decides what it means. The
   *  surface that needs it is the sideload notice: an iOS build is signed with the user's own
   *  Apple ID and stops opening about seven days later. */
  platform: string;
}

/** What asking a remote — without cloning it — told us. The three states need three different
 *  next steps, which is the whole reason this exists rather than showing git's raw stderr. */
export interface RemoteProbe {
  state: 'reachable' | 'needs_auth' | 'unreachable';
  /** One sentence naming what to do next; git's own words when we did not recognise the error. */
  detail: string;
  /** The credential helper's program name, or null. **Never a secret.** */
  helper: string | null;
  /** The configured helper keeps credentials in plaintext (`store` does). */
  helper_is_plaintext: boolean;
}

/** Where git credentials live on this machine. */
export interface GitAuth {
  /** `system` = git's helper owns it and we store nothing; `app` = no helper here, so
   *  formicaria keeps the token; `none` = no git at all. */
  storage: 'system' | 'app' | 'none';
  have_credential: boolean;
  helper: {
    configured: string | null;
    /** Credentials are kept in plaintext on disk. */
    plaintext: boolean;
    /** A better helper that is actually installed here, or null. */
    better: string | null;
  } | null;
}
