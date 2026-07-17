// Mirrors the Rust DTOs in crates/fm-app/src/dto.rs. `props` is an open map, so
// a custom frontmatter property reaches the UI with no type change.
export interface ObjectMeta {
  id: string;
  type: string;
  title: string | null;
  preview: string;
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
}

/** What each backup tier could do right now (`backup_status`). */
export interface BackupStatus {
  /** Every vault, in configured order; the first is the default for new notes. A
   *  single-vault install is a list of one. */
  vaults: VaultStatus[];
  /** The restic repo — a path or URL, never the password. One repo for the whole set:
   *  restic is disaster recovery, not distribution, so unlike git it does **not**
   *  respect vault boundaries. The panel says so. */
  restic_repo: string | null;
  /** Both restic env vars present, i.e. a full backup could actually run. */
  restic_ready: boolean;
}

/** What a pull did. Conflicts are a result, not a failure. */
export interface PullResult {
  merged: number;
  conflicts: string[];
}
