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

/** What each backup tier could do right now (`backup_status`). */
export interface BackupStatus {
  /** Where the notes push to, or null when no remote is set yet. */
  remote: string | null;
  /** Commits made here but not on the remote; null when never pushed. */
  unpushed: number | null;
  /** The restic repo — a path or URL, never the password. */
  restic_repo: string | null;
  /** Both restic env vars present, i.e. a full backup could actually run. */
  restic_ready: boolean;
}
