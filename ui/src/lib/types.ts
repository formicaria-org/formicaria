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
  /** How it stands (2026-10-24): open, merged, declined or closed. */
  state: 'open' | 'merged' | 'declined' | 'closed';
  /** When it closed (RFC3339), when recorded. */
  closed_at: string | null;
  /** A rejection's reason, in the reviewer's words. */
  why: string | null;
  /** For a closed one: `files`/`patch` are what it did, read from history. */
  shown_from_history: boolean;
}

/** The proposed note behind a proposal — for the review to show and edit before accepting. */
export interface ProposalContent {
  /** The note this proposal edits (an edit is saved back through `create_proposal` on this note). */
  host: string;
  title: string;
  /** The proposed note body as it stands on the branch. */
  body: string;
  /** When/where the proposed note says it happens — shown in plain view, not only in the diff. */
  start?: string | null;
  due?: string | null;
  location?: string | null;
  /** The same three as the note has them now — what each proposed value would replace. */
  wasStart?: string | null;
  wasDue?: string | null;
  wasLocation?: string | null;
  /** The note's title now — a proposal that renames a conversation shows what it renames from. */
  wasTitle?: string | null;
  /** The proposal adds a note that does not exist yet (a meeting found in an email exchange). */
  isNew?: boolean;
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
  /** Someone else has pushed work we don't have. Null when unknowable: no remote,
   *  never pushed, or simply offline — a sleeping laptop is not an error. */
  remote_moved: boolean | null;
  /** Notes with conflict markers in them, waiting for a human. The `.md` merge driver
   *  keeps markers out of the frontmatter, so these still open in the editor. */
  conflicts: string[];
  /** This vault's media could actually be backed up **now**: restic is installed, this
   *  vault has a repo, and the password is set. All three — "ready" has to mean
   *  "will work", not "is configured". */
  restic_ready: boolean;
  /** Attachments this vault will put in the next commit, and their total bytes. The rule selects
   *  every blob at or under `git_assets_max` — not a diff against the remote — so a device that
   *  has never sent one stages its whole backlog the first time it can. */
  assets_pending: number;
  assets_pending_bytes: number;
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
  /** When anything was last saved *here*, epoch seconds; `null` = never saved. */
  last_commit: number | null;
  /** When this vault's notes last *left the device*, epoch seconds; `null` = never sent.
   *  Not the same clock as `last_commit`, and the gap between them is the point: the
   *  auto-save loop keeps the first fresh whether or not anything is reaching a backup. */
  last_sent: number | null;
  /** How much has been saved here and never sent. `null` — not `0` — when there is nothing
   *  to count against, which is a different state from "all sent". */
  unsent: number | null;
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
  /** The clock this vault's times are kept on (`Asia/Tokyo`, `+09:00`): stated in its
   *  `vault.json`, else read from its first commit. `null` when it has neither. Each device shows
   *  the times on its own zone; this says what they are converted from. */
  clock?: string | null;
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
  /** This vault's snapshot repository, from its config entry. Here rather than only on
   *  `VaultStatus` because it costs nothing to read and the settings screen needs it without
   *  paying for a network round trip — it used to arrive as `Config.restic`, a separate list the
   *  panel joined back by name. */
  restic_repo: string | null;
  /** **What to show instead of `name`**: the repository behind this vault's remote
   *  (`…/formicarium-vault.git` → `formicarium-vault`), or `null` for a vault with no remote — or
   *  when two vaults would derive the same label. Display only; `name` stays the identity. See
   *  `vaults.svelte.ts`. */
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
  /** The attachments by kind (extension), largest first — what the person may leave out. */
  kinds: AttachmentKind[];
  /** Excalidraw drawings, which become whiteboard notes. */
  drawings: number;
  /** Vaults that already hold most of this folder's pages, from an earlier import. */
  previous: PreviousImport[];
}

export interface AttachmentKind {
  kind: string;
  count: number;
  bytes: number;
}

export interface PreviousImport {
  vault: string;
  /** How many of the folder's pages it holds, of how many. */
  pages: number;
  of: number;
}

/** The person's rules for an import (`decisions.md#data`, 2026-10-15). Absent = add only, and
 *  bring every attachment. */
export interface ImportRules {
  /** Rewrite what an earlier import wrote, except notes edited or deleted since. */
  replace?: boolean;
  /** Leave out attachments larger than this many MB. */
  maxMb?: number | null;
  /** Leave out attachments with these extensions. */
  skipKinds?: string[];
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
  /** Attachments the rules left out — each still named where a note mentions it. */
  leftOut: number;
  leftOutNames: string[];
  /** Links to pages with no file of their own, now tags. */
  tagsFromLinks: number;
  /** Excalidraw drawings, now whiteboard notes. */
  drawings: number;
  /** On a replace: earlier notes rewritten, and earlier notes kept because they were edited or
   *  deleted since. */
  replaced: number;
  keptEdited: number;
  keptEditedNames: string[];
}

/** The renderer a `.view` draws through — the same set the built-in nav offers. */
export type Renderer = 'board' | 'agenda' | 'timeline' | 'search' | 'gallery' | 'totals' | 'shelf';

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
  /** The group this view belongs to (`decisions.md#ui`, 2026-10-11). Absent for an ordinary view. */
  group?: string;
  /** Came with the app, not from a vault. Whether its group is shown is this device's choice. */
  shipped?: boolean;
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
  /** What a `totals` view sums and how it names it. The data itself comes from `log_days`. */
  totals?: TotalsDef;
  /** The view's group and its sibling views, in order. It travels with the payload, like `filters`. */
  group?: ViewGroup;
}

// ── Trackers (`decisions.md#data`, `#ui`, 2026-10-11) ────────────────────────────────────────────
// A logged entry is a note with a reserved `log` property, a day in `start`, a `slot`, and whole
// numbers whose unit is in the key (`kcal`, `protein_mg`). These are the shapes its commands answer.

/** One value a view sums. `role` decides how prominently it is drawn; the unit comes from the key. */
export interface Metric {
  key: string;
  label: string;
  role: 'primary' | 'secondary' | 'detail';
  /** How a day's entries become the day's value: added up (the default), or averaged — a weight
   *  taken twice in a day is not two weights. */
  agg?: 'sum' | 'mean';
  /** The log this value is read from, when it is not the view's own. */
  log?: string;
}
/** A part of the day an entry belongs to — a meal, for a food log. */
export interface Slot {
  value: string;
  label: string;
}
export interface TotalsDef {
  /** Which entries: the value of their `log` property. */
  log: string;
  /** Which of the closed set of add actions the group's add control opens. */
  add: 'food' | 'workout' | 'body' | 'health';
  /** `plan` is the screen where a diet or a regime is chosen and followed; `diary` and `month` are
   *  the Health diary's (2026-10-22). */
  span: 'day' | 'range' | 'plan' | 'menu' | 'week' | 'cards' | 'diary';
  metrics: Metric[];
  slots: Slot[];
}
export interface ViewGroup {
  name: string;
  views: string[];
}

/** **What kind a Health tracker is**, read off the end of its key (`health.rs`'s `parse_key`). */
export type TrackerKind =
  | { kind: 'severity' }
  | { kind: 'rating' }
  | { kind: 'bother' }
  | { kind: 'yesno' }
  | { kind: 'count' }
  | { kind: 'amount'; unit: string }
  | { kind: 'score' };
/** One thing a person tracks in the Health diary: a key on their Health template. */
export type Tracker = { key: string; label: string } & TrackerKind;
/** One day of the diary. A key absent from `values` was not recorded that day. */
export interface HealthDay {
  day: string;
  id?: string;
  values: Record<string, number>;
  /** Written more than a day after the day it is about. */
  late?: boolean;
  /** The cycle's context set or cleared on this day: the person's words, or `none`. */
  context?: string;
  /** What fm already holds for the day, read not typed: `water_mg`, `caffeine_ug`, `alcohol_mg`
   *  from Food's log; `weight_g` (a mean) and `steps` from the body log. */
  elsewhere?: Record<string, number>;
}
/** The record, and the kinds that already have a template. */
export interface HealthRecord {
  items: RecordItem[];
  templates: string[];
}
/** One item of the health record (`record.rs`): an ordinary note with a `record` kind. */
export interface RecordItem {
  id: string;
  kind:
    | 'medicine'
    | 'allergy'
    | 'condition'
    | 'result'
    | 'visit'
    | 'immunization'
    | 'procedure'
    | 'device'
    | 'contact'
    | 'none_known'
    | 'thought'
    | 'worry'
    | 'writing';
  title: string;
  status?: string;
  start?: string;
  due?: string;
  about?: string;
  /** Its other properties as written: a dose, a reaction, a result's value and the lab's range. */
  fields: Record<string, string>;
}
/** A span's counts for one tracker. Nothing here labels a day. */
export interface HealthCount {
  of: number;
  recorded: number;
  /** Days at or above 1. */
  days: number;
  /** Days at or above 2, for a severity: IHS's *moderate or severe*. */
  moderate_or_severe?: number;
  /** Values not counted: not a whole number the tracker's kind accepts. */
  odd?: number;
}
/** A sentence a Health card carries, and whose. `nhs` marks the NHS website's own words. */
export interface HealthSaid {
  by: string;
  text: string;
  nhs?: boolean;
  /** In our own words, not the source's: shown as ours, never as a quotation. */
  paraphrase?: boolean;
}
/** One way a Health card counts the diary (`guidance.rs`'s closed set). */
export interface HealthCheck {
  id: string;
  label: string;
  kind:
    | 'kept'
    | 'week_days'
    | 'month_days'
    | 'days_over'
    | 'at_least'
    | 'run'
    | 'cycle_lengths'
    | 'bleeding_days'
    | 'cycle_spread'
    | 'spotting_days'
    | 'heavy_days'
    | 'around_period'
    | 'cycles_kept'
    | 'food_mean'
    | 'bp_home'
    | 'questionnaire';
  keys?: string[];
  keys_from?: string[];
  value?: number;
  window?: number;
  min?: number;
  max?: number;
  at_or_over?: number;
  max_by_age?: [number, number, number][];
  min_female?: number;
  min_male?: number;
  max_second?: number;
  basis: string;
}
export interface HealthCard {
  id: string;
  shelf: 'diary' | 'nose' | 'daily' | 'cycle' | 'mind' | 'own';
  name: string;
  rests_on: 'guideline' | 'consensus' | 'review' | 'definition' | 'own';
  says: HealthSaid[];
  urgent?: HealthSaid[];
  disagree?: string;
  sources: { by: string; title: string; url: string }[];
  trackers: string[];
  choose?: { id: string; label: string }[];
  /** A questionnaire carried word for word, its holder allowing it: answers score 0, 1, 2 … */
  form?: { prompt: string; answers: string[]; items: { key: string; text: string }[] };
  checks: HealthCheck[];
  not_said: string[];
}
/** A check counted on a day: the person's count beside the body's number. */
export interface HealthCounted {
  id: string;
  label: string;
  basis: string;
  value: number;
  of: number;
  recorded: number;
  from: string;
  to: string;
  min?: number;
  max?: number;
  at_or_over?: number;
  within?: boolean;
  unchosen?: boolean;
  /** The numbers behind the count: cycle lengths. */
  values?: number[];
  /** The cycles counted over; `left_out` says why one is not counted. */
  cycles?: HealthCycle[];
  /** Headache days outside every period's window. */
  outside?: number;
  /** Why the count is not set beside the number. */
  not_compared?: string;
}
/** One cycle, first day to first day (`cycle.rs`). */
export interface HealthCycle {
  start: string;
  length: number;
  noted: number;
  note: string;
  left_out?: string;
}
export interface HealthFollowed {
  card: string;
  note: string;
  since: string;
  chosen: Record<string, string[]>;
  checks: HealthCounted[];
}
export interface HealthGuidance {
  nhs_credit: string;
  cards: HealthCard[];
  followed: HealthFollowed[];
}
/** One ISO week of one tracker. A mean is `sum / recorded`; a part week says how many days it had. */
export interface HealthWeek {
  /** The Monday it starts on. */
  week: string;
  of: number;
  recorded: number;
  days: number;
  sum: number;
}
export interface HealthDays {
  /** The Health template a new tracker is written into; null until something is tracked. */
  template: string | null;
  trackers: Tracker[];
  days: HealthDay[];
  counts: Record<string, HealthCount>;
  /** Per tracker, its weeks, when asked for. */
  weeks?: Record<string, HealthWeek[]>;
}
/** A sum, and how many of the day's entries carried the value at all. */
export interface MetricSum {
  sum: number;
  n: number;
  /** How many entries carried something that is not a whole number. Absent when none did. */
  odd?: number;
  /** The least and the most the sum could be, when some entry's amount is a range. */
  low?: number;
  high?: number;
}
export interface DayTotals {
  /** `YYYY-MM-DD`. */
  day: string;
  /** How many entries the day has. */
  of: number;
  sums: Record<string, MetricSum>;
  /** The targets in force that day, by metric key. Empty when there is no program. */
  targets: Record<string, number>;
  rows?: ObjectMeta[];
  /** When the day's entries were, for those that carry a time. Entries less than 15 minutes
   *  apart are one occasion. */
  times?: { occasions: number; first: string; last: string };
}
export interface LogDays {
  days: DayTotals[];
}
/** Something logged before, offered again with the amount last used. */
export interface RecentItem {
  title: string;
  props: Record<string, unknown>;
  count: number;
  last: string;
}
/** A saved recipe and what its ingredients come to (`log_recipes`). */
export interface Recipe {
  /** The word its notes share; an entry logged from it records `recipe/<this>` as its source. */
  recipe: string;
  id: string;
  title: string;
  servings: number;
  /** How many ingredients it has. */
  of: number;
  /** The whole recipe, per value: the sum, and how many ingredients carried it. */
  sums: Record<string, MetricSum>;
  ingredients: ObjectMeta[];
  /** The carried recipe it was made from — e.g. `nhlbi/<id>` — when it was. */
  from?: string;
}
/** One vitamin or mineral of a day beside its reference (`log_nutrients`). */
export interface NutrientRow {
  key: string;
  label: string;
  sum: number;
  /** How many of the day's entries carried the value. */
  n: number;
  /** The reference for this person, in the key's own unit, when there is one figure. */
  reference?: number;
  /** `PRI` or `AI` in EFSA's terms; empty when the row has only a limit. */
  kind: string;
  upper?: number;
  note?: string;
}
export interface Nutrients {
  day: string;
  of: number;
  rows: NutrientRow[];
  /** The person's sex is not known, so references that differ by it are left out. */
  needs_sex: boolean;
  entries: ObjectMeta[];
}
/** A packaged food written down from its label (`log_products`). */
export interface Product {
  id: string;
  title: string;
  barcode?: string;
  serving_g?: number;
  /** A serving of a food whose label is by volume. */
  serving_ml?: number;
  /** `ml` when the food is measured in millilitres; absent for grams. */
  unit?: string;
  /** What a hundred of its unit hold, by the keys every food uses. */
  per100: Record<string, number>;
}
/** A product as Open Food Facts gave it, shaped as the label form's fields (`product_lookup`). */
export interface ProductFound {
  name: string;
  barcode: string;
  serving_g?: number;
  per100: Record<string, number>;
  ingredients: string;
  /** The product's own page there, to link to and to credit. */
  page: string;
}
/** A recipe carried in the app, as its source published it (`recipe_library`). */
export interface CarriedRecipe {
  id: string;
  title: string;
  /** Its own page, where the source has one. */
  url?: string;
  /** The source's own words for what it makes. */
  yield?: string;
  servings?: number;
  /** Its ingredient lines, as printed, where the source lists them. */
  ingredients?: string[];
  steps?: string[];
  /** What the page prints under its directions: a tip, a note, a variation. */
  notes?: string[];
  /** The source's own values for a serving, by the keys every food uses — where it gives any. */
  per_serving?: Record<string, number>;
  /** A book's own: the recipe's number, its paragraphs, and the recipes it sends you to. */
  number?: number;
  text?: string[];
  refs?: number[];
  /** The source's own time fields, in its words ("preparazione": "20 minuti"). */
  times?: Record<string, string>;
  /** The region it names, when it names one. */
  region?: string;
  /** A link to a photo — never fetched or carried. */
  image?: string;
  /** Who wrote it, where the licence asks them to be credited (a wiki page's history). */
  authors_url?: string;
}
/** A recipe read from a page a person chose (`recipe_from_link`): each field the page's own. */
export interface WebRecipe {
  title: string;
  url: string;
  site?: string;
  author?: string;
  ingredients: string[];
  steps: string[];
  yield_text?: string;
  servings?: number;
  prep_minutes?: number;
  cook_minutes?: number;
  total_minutes?: number;
  /** A link to the page's photo; never fetched. */
  image?: string;
}
export interface RecipeLibrary {
  /** What a logged serving's source starts with: `<key>/<id>`. */
  key: string;
  /** What each of its recipes is labelled with in the one recipe search. */
  label: string;
  source: string;
  home: string;
  seen: string;
  /** Where the text was taken from, cited. */
  taken_from?: string;
  language?: string;
  /** The collection's licence, where it is not public domain (Wikibooks: CC BY-SA 4.0). */
  licence?: string;
  licence_url?: string;
  recipes: CarriedRecipe[];
}
/** A moment a reminder is due (`reminders`), in the notes' own wall-clock time. */
export interface Reminder {
  /** The moment in milliseconds, worked out by the server on the app's clock (2026-10-24). */
  at_ms?: number;
  id: string;
  title: string;
  /** `YYYY-MM-DDTHH:MM`. */
  at: string;
  what: 'start' | 'due';
  /** The note's own time, when the reminder is earlier than it. */
  when?: string;
  all_day: boolean;
}
/** A session kept under a name, to plan again on the days it is meant for (`log_routines`). */
export interface Routine {
  /** The word its notes share. */
  routine: string;
  id: string;
  title: string;
  /** `mon` … `sun`; empty when it is meant for no day in particular. */
  weekdays: string[];
  sets: ObjectMeta[];
}
/** A note to create. Several are created in one write (`create_notes`). */
export interface NoteDraft {
  title: string;
  body?: string;
  tags?: string[];
  start?: string;
  props: Record<string, string | number>;
  vault?: string;
}
export interface FoodHit {
  id: string;
  name: string;
  /** Which table the food is from: `ciqual`, `bls`, `swiss`, `crea` or `usda`. */
  src: string;
  /** Energy per 100 g. */
  kcal: number;
  /** The name it was found by in its own table's language, when not the English one. */
  alt?: string;
}
export interface FoodPortion {
  label: string;
  grams: number;
}
export interface FoodDetail {
  id: string;
  name: string;
  src: string;
  /** The food's group in its own table. Empty where the table gives none. */
  grp: string;
  /** Every value per 100 g, keyed like an entry's properties. */
  per100: Record<string, number>;
  portions: FoodPortion[];
  /** The pack and version the values came from; an entry records it as its `source`. */
  pack: string;
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

/** A vault folder on this device that is **not** in the vault list — something to take back with
 *  one tap. Only ever non-empty where there is a managed vault root (the phone, iOS): a desktop
 *  puts vaults anywhere and has a file picker, so there is no closed set to offer.
 *
 *  `name` is the folder, which is also what to call the vault to land back on it — on a phone the
 *  name *is* the address. `notes` is how many Markdown files are in it, because the question being
 *  asked is never "which folder" but "which one has my work in it". */
export interface Recoverable {
  name: string;
  path: string;
  notes: number;
}

/** One step of a staged backup: send every attachment at or under `cap`.
 *  Cumulative by construction — a step includes everything the steps before it did, which is what
 *  makes each push carry only the difference. */
export interface AssetBatch {
  cap: number;
  count: number;
  bytes: number;
}

/** What a reference pack says about itself (`fm_core::pack::PackInfo`). */
export interface PackInfo {
  name: string;
  version: string;
  title: string;
  licence: string;
  attribution: string;
  foods: number;
}
/** The food list this app carries: what it says about itself and what it takes on this device. */
export interface PackStatus {
  /** `null`, with `error`, when it could not be put in place. */
  installed: PackInfo | null;
  bytes: number;
  error?: string;
}

// ── diets and regimes to follow (`plan.rs`; `decisions.md#data`, 2026-10-12) ─────────────────────
export interface PlanSource {
  by: string;
  title: string;
  url: string;
}
export interface PlanGuide {
  label: string;
  by: string;
  url: string;
  /** The day it was last seen to work. */
  seen: string;
}
/** One thing a plan holds a person to. `kind` says how it is measured. */
export interface PlanCheck {
  id: string;
  label: string;
  kind:
    'sum' | 'share' | 'low_days' | 'minutes' | 'days' | 'sets' | 'region_days' | 'kept' | 'fast';
  key?: string;
  /** The log its entries are read from, when not the group's own (`body` for steps). */
  log?: string;
  min?: number;
  max?: number;
  /** Flags under which the number is not shown — set by a clinician for that person. */
  hide_if?: string[];
  /** Whose number it is, in the source's words. */
  basis: string;
}
/** What a person may tell the app about themselves — opt-in, never required (`plan.rs` `FLAGS`). */
export type PlanFlag =
  'pregnant' | 'breastfeeding' | 'postmenopausal' | 'kidney' | 'diabetes_meds' | 'eating_disorder';
/** What kind of thing a card is, which says how it is tracked (`decisions.md#ui`, 2026-10-16). */
export type PlanKind = 'targets' | 'pattern' | 'timing' | 'amounts' | 'programme' | 'own';
export interface PlanOption {
  id: string;
  group: 'food' | 'fitness';
  name: string;
  /** What kind of source it rests on. */
  rests_on: 'guideline' | 'position' | 'review' | 'definition' | 'described' | 'own';
  summary: string;
  sources: PlanSource[];
  needs: string[];
  checks: PlanCheck[];
  describes?: string[];
  unmeasured: string[];
  caution?: string;
  guides: PlanGuide[];
  /** A week (a regime) or a day (a diet) to start from, to be changed. */
  examples?: (PlanExample | PlanFoodExample)[];
  kind: PlanKind;
  /** The question it answers; one card is followed per question. */
  question: string;
  /** The card to start from: written for all adults. */
  start_here?: boolean;
  /** How it relates to the other cards of its question. */
  relation?: string;
  /** Flags under which it is not offered. */
  not_for?: PlanFlag[];
  /** What changes for a person who has set a flag, and on whose word. */
  flag_notes?: { flag: PlanFlag; text: string }[];
}
export interface PlanExampleItem {
  type: 'activity' | 'session';
  name: string;
  minutes?: number;
  effort?: number;
  counts_as?: string;
  exercises?: { card: string; sets: number; reps: string | number; said?: string }[];
  said?: string;
}
/** A diet's day to start from: a body's own, or chosen from the carried recipes. */
export interface PlanFoodExample {
  kind: 'published' | 'selected';
  by: string;
  title: string;
  url: string;
  note: string;
  /** Published: the day as printed. */
  day?: {
    name?: string;
    meals: { meal: string; items: string[] }[];
    totals?: Record<string, string>;
  };
  fits?: string;
  /** Selected: carried recipes by id, a serving each. */
  recipes?: { meal: string; recipe: string; servings: number }[];
  totals?: Record<string, number>;
  against?: string;
}
export interface PlanExample {
  /** `published`: a body's own example. `assembled`: the app's arrangement of sourced figures. */
  kind: 'published' | 'assembled';
  variant?: string;
  by: string;
  title: string;
  url: string;
  note: string;
  week: { day: string; items: PlanExampleItem[] }[];
  meets?: string;
  later?: string;
}
export interface PlanActivity {
  name: string;
  kind: string;
  by: string;
  /** A place to start from, where a source gives one, and whose it is. */
  minutes?: number;
  effort?: number;
  basis?: string;
}
/** A week laid out day by day by a public body, word for word. */
export interface ExampleWeeks {
  by: string;
  title: string;
  url: string;
  seen: string;
  examples: { name: string; about: string; days: string[]; total: string }[];
}
export interface PlanLibrary {
  plans: PlanOption[];
  activities: PlanActivity[];
  /** Fitness only. Sunday to Saturday, as the source lists them. */
  weeks?: ExampleWeeks[];
  /** Cards left out because of what the profile says, by id. */
  hidden?: string[];
}
/** How an exercise is done, as its source wrote it (`exercise_library`). */
export interface ExerciseSource {
  id: string;
  by: string;
  credit: string;
  /** NHS: the day the text was copied, `DDMMYY`, which its terms ask to be shown. */
  as_at?: string;
  licence: string;
  licence_url?: string;
  home: string;
}
export interface ExerciseCard {
  id: string;
  source: string;
  name: string;
  url: string;
  ref: string;
  group: string;
  equipment: 'none' | 'chair or wall' | 'bar' | 'free weights' | 'machine' | 'other';
  equipment_said?: string;
  area: string;
  about?: string;
  start?: string;
  steps: string[];
  dose?: string;
  notes?: string[];
}
export interface ExerciseVideo {
  exercise: string;
  url: string;
  title: string;
  channel: string;
  /** `body`: a known body's own channel. `expert`: a trainer whose science or clinical
   *  qualification was confirmed. `trainer`: an individual with many views and nothing confirmed. */
  tier: 'body' | 'expert' | 'trainer';
  /** For `expert`: the qualification, in its source's words, and where it was read. */
  credential?: string;
  credential_url?: string;
  /** For a body's channel: was it confirmed from the body's own website, or only by its name? */
  confirmed?: boolean;
  views?: number | null;
  seconds?: number | null;
  note?: string;
  /** The cards it is of, by id; or the drills, by the group their cards carry. */
  cards?: string[];
  groups?: string[];
}
export interface ExerciseLibrary {
  seen: string;
  sources: ExerciseSource[];
  exercises: ExerciseCard[];
  videos: ExerciseVideo[];
  videos_seen?: string;
  /** The NIA's Safety Tips box, whole, for the cards that need `for`'s equipment. */
  safely?: ExerciseSafety;
  /** Pages elsewhere on how an exercise is done, by card id: their titles only. */
  links?: Record<string, ExerciseLink[]>;
  links_seen?: string;
}
export interface ExerciseSafety {
  title: string;
  by: string;
  credit: string;
  /** The publication the box is from. */
  work: string;
  url: string;
  licence: string;
  licence_url: string;
  for: ExerciseCard['equipment'][];
  tips: { head: string; text: string }[];
}
export interface ExerciseLink {
  title: string;
  url: string;
  by: string;
}
/** A person's measurements, as far as they have given them. */
export interface PlanProfile {
  id?: string;
  sex?: 'female' | 'male';
  born?: number;
  height_cm?: number;
  weight_g?: number;
  /** EFSA's activity level times ten: 14, 16, 18 or 20. */
  pal_x10?: number;
  /** The opt-in flags the person has set. */
  flags?: PlanFlag[];
}
export interface PlanBound {
  min?: number;
  max?: number;
}
export interface PlanPreview {
  plan: PlanOption;
  profile: PlanProfile;
  missing: string[];
  energy?: number;
  bounds: Record<string, PlanBound>;
}
export interface PlanCheckProgress {
  id: string;
  label: string;
  kind: PlanCheck['kind'];
  /** A value's own key, or `percent`, `minutes`, `days`, `sets`. */
  unit: string;
  per: 'day' | 'week';
  min?: number;
  max?: number;
  value?: number;
  state: 'met' | 'under' | 'over' | 'none';
  /** Monday to Sunday. */
  days: (number | null)[];
  basis: string;
  /** The last four weeks, oldest first, the last this week so far: a day check's average over the
   *  days that had something, a week check's total; `null` for a week with nothing logged. */
  trend?: (number | null)[];
}
/** One followed card, measured: there is one per question. */
export interface PlanFollowed {
  question: string;
  plan?: PlanOption;
  note: ObjectMeta;
  unknown?: string;
  checks: PlanCheckProgress[];
  kept_today?: string;
}
/** The "worth watching" set: measured whichever card is followed, or none. */
export interface PlanWatch {
  from: string;
  to: string;
  checks: PlanCheckProgress[];
}
export interface PlanProgress {
  plan?: PlanOption;
  note?: ObjectMeta;
  /** The note names a plan this version does not have. */
  unknown?: string;
  from: string;
  to: string;
  day: string;
  checks: PlanCheckProgress[];
  /** The id of the note saying the person kept to it today. */
  kept_today?: string;
  /** Every card followed in the group, one per question; the fields above are the first. */
  followed?: PlanFollowed[];
}
