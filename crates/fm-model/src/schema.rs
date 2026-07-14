//! On-disk format version. Frozen early and migrated forward — the files must
//! outlive every app rewrite, so the format is versioned from commit 1.

/// The current on-disk schema version, written into each note's frontmatter as
/// `schema: N`.
pub const SCHEMA_VERSION: u32 = 1;

/// Whether a file written under `file_schema` predates the current version and
/// must be migrated on read. The migration hook exists now so a future format
/// change is a data migration, never an app rewrite.
pub fn needs_migration(file_schema: u32) -> bool {
    file_schema < SCHEMA_VERSION
}
