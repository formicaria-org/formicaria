// Mirrors the Rust DTOs in crates/fm-app/src/dto.rs. `props` is an open map, so
// a custom frontmatter property reaches the UI with no type change.
export interface ObjectMeta {
  id: string;
  type: string;
  title: string | null;
  preview: string;
  status: string | null;
  due: string | null;
  hard: boolean;
  created: string;
  updated: string;
  tags: string[];
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
