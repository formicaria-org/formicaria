//! FileStore: markdown files are the truth; SQLite is the disposable per-machine
//! index, rebuilt from the files on open. It implements the same `Store` contract
//! as `MemoryStore`, so the two are interchangeable behind the seam — a test
//! asserts they return equivalent query results.
//!
//! Two tables: `objects` holds the canonical file content; `fts` is a SQLite
//! FTS5 full-text index over each object's `searchable_text()`. A `Text`
//! predicate is answered by an FTS5 `MATCH` that loads only the hits; the pure
//! engine then applies the rest of the query over that candidate set. Every
//! other query (no `Text`) loads all objects and runs the engine — identical to
//! MemoryStore. This is the storage half of "full-text search is just a
//! predicate": search is fast, but not special.

use crate::frontmatter;
use crate::{Reindex, ReindexStats, Store, StoreError};
use fm_model::{Id, Object};
use fm_query::{Filter, Predicate};
use rusqlite::{Connection, OptionalExtension};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct FileStore {
    notes: PathBuf,
    db: Connection,
    skipped: Vec<crate::SkippedNote>,
    /// What this vault is called — the audience every note here belongs to. Stamped
    /// onto each object on the way out, because `Object.vault` is derived from
    /// location and never read from a file.
    name: String,
    /// What this vault is for, from its `vault.json`. `None` when it has no descriptor —
    /// which is most vaults, and not a gap to fill in with something invented.
    description: Option<String>,
    /// Every note file **this app** has written or deleted since the last commit.
    ///
    /// The auto-commit stages exactly these. Staging a directory instead meant that in a
    /// vault which is also a project — the direction Track V is heading — a note you were
    /// hand-editing in Vim got committed mid-sentence by a debounce five seconds later.
    /// `put` is the only thing that knows which file it just wrote, so it is the only thing
    /// that can answer this honestly.
    ///
    /// **Deliberately not on the `Store` trait.** That seam carries no paths, no mtimes and
    /// no directory handles, and it is the reason a storage swap stays a backend change.
    /// `MultiStore` reaches these through the concrete type instead.
    written: std::collections::BTreeSet<PathBuf>,
}

impl FileStore {
    /// Open (creating if needed) a vault at `root`, then rebuild the index from
    /// the files on disk. The index is disposable: delete `index.sqlite`,
    /// reopen, and it reconstructs exactly.
    ///
    /// The vault takes its name from its directory — see [`FileStore::named`] when
    /// something else has an opinion (a `MultiStore` reading the vault list).
    pub fn open(root: impl AsRef<Path>) -> Result<Self, StoreError> {
        // Empty means "no opinion", which lets `named` apply the real precedence:
        // an explicit caller name, then the vault's own `vault.json`, then the directory.
        // Passing the directory name here would make it an *opinion* and the descriptor
        // could never win — which is backwards, since the directory name is the weakest
        // signal of the three (it is whatever git called the clone).
        Self::named(root, "")
    }

    /// Open a vault under a name the caller chooses. That name *is* the audience
    /// label users see and filter on, so it belongs to whoever configured the vault
    /// list, not to whatever the directory happens to be called.
    pub fn named(root: impl AsRef<Path>, name: impl Into<String>) -> Result<Self, StoreError> {
        let root = root.as_ref();
        // `<vault>/vault.json`, if the vault has an opinion. Absent is the common case and
        // means exactly today's behaviour: notes in `notes/`, name from the caller.
        //
        // The caller's name still wins when it gave one — that is the vault *list*, i.e. the
        // audience label the person running this app chose, and a repo they cloned does not
        // get to rename their audience out from under them. The descriptor supplies it only
        // when nobody else did.
        let desc = crate::descriptor::Descriptor::read(root)?;
        let notes = desc.notes_dir(root);
        // Caller > descriptor > directory. The caller is the vault *list* — the audience
        // label this user chose — so a repo they cloned never renames it out from under
        // them. The directory is the last resort: it is whatever git called the clone.
        let name = {
            let given: String = name.into();
            if !given.is_empty() {
                given
            } else {
                desc.name.clone().unwrap_or_else(|| {
                    root.file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "vault".to_string())
                })
            }
        };
        fs::create_dir_all(&notes).map_err(io)?;
        let db = Connection::open(root.join("index.sqlite")).map_err(sql)?;
        let mut store =
            FileStore {
                notes,
                db,
                skipped: Vec::new(),
                name,
                description: desc.description,
                written: Default::default(),
            };
        store.init_schema()?;
        let stats = store.reindex(Reindex::Full)?;
        store.skipped = stats.skipped;
        Ok(store)
    }

    /// This vault's name — its audience.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The note files this app has written or deleted since [`clear_written`] was last
    /// called. Borrowed rather than drained: a commit that fails must not lose the list,
    /// and staging a file twice is a no-op.
    ///
    /// [`clear_written`]: FileStore::clear_written
    pub fn written(&self) -> Vec<PathBuf> {
        self.written.iter().cloned().collect()
    }

    /// Forget the write list — called only after a commit has actually succeeded.
    pub fn clear_written(&mut self) {
        self.written.clear();
    }

    /// What this vault is for, if it says. Never invented: a vault with no `vault.json` has
    /// no description, and "" would be a claim we cannot support.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Notes the last reindex could not read, as `filename: why`. The vault serves
    /// everything else, which is the whole point — but the caller **must** say so,
    /// because a note that silently vanished from every view is a worse failure than
    /// the startup crash this replaced. Loud is recoverable; silent is not.
    pub fn skipped(&self) -> &[crate::SkippedNote] {
        &self.skipped
    }

    fn init_schema(&self) -> Result<(), StoreError> {
        // `objects` = canonical content; `fts` = full-text over searchable_text().
        // The tokenizer is pinned (`unicode61 remove_diacritics 2`) so search
        // folds case and accents the same way on every machine — the guard
        // against FileStore/MemoryStore query drift (MASTERPLAN risk #5).
        self.db
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS objects (
                     id        TEXT PRIMARY KEY,
                     path      TEXT NOT NULL,
                     mtime_ns  INTEGER NOT NULL,
                     content   TEXT NOT NULL
                 );
                 -- **`path` is queried, so it is indexed.** `forget_path` deletes by path on
                 -- every write and once per changed note in a reindex; without this SQLite
                 -- full-scans `objects` each time, which makes a rebuild O(n²). Measured at
                 -- 10 000 notes: 54.7 s before, and the growth was ~5x per doubling where linear
                 -- would be 2x. `IF NOT EXISTS`, so an index built by an older version gains it
                 -- on the next open — which is free, because the index is disposable anyway.
                 CREATE INDEX IF NOT EXISTS objects_path ON objects(path);
                 CREATE VIRTUAL TABLE IF NOT EXISTS fts USING fts5(
                     id UNINDEXED,
                     text,
                     tokenize = 'unicode61 remove_diacritics 2'
                 );",
            )
            .map_err(sql)
    }

    fn path_for(&self, id: Id) -> PathBuf {
        self.notes.join(format!("{id}.md"))
    }

    /// Write via temp-file + rename so the canonical file is never half-written.
    /// The temp lives in the same directory, so the rename is atomic.
    fn write_atomic(path: &Path, contents: &str) -> Result<(), StoreError> {
        let tmp = path.with_extension("tmp");
        {
            let mut f = fs::File::create(&tmp).map_err(io)?;
            f.write_all(contents.as_bytes()).map_err(io)?;
            f.sync_all().map_err(io)?;
        }
        fs::rename(&tmp, path).map_err(io)?;
        Ok(())
    }

    /// Index one object into both tables: the `objects` row (canonical content +
    /// mtime) and the `fts` full-text row. The FTS table is plain (not
    /// external-content), so a refresh is delete-then-insert. `searchable_text()`
    /// decides what's searchable — title + body today, + extracted asset text
    /// (pdftotext) once ingest lands in S5.
    /// Write one note's row and its full-text entry.
    ///
    /// `fresh` means **"the caller knows `fts` holds no row for this id"**, which is true for
    /// exactly one caller: a `Reindex::Full`, which empties both tables before the loop starts.
    ///
    /// That flag is not a micro-optimisation, it is the difference between linear and quadratic.
    /// `fts` is declared `id UNINDEXED` — in FTS5 that stores the column **without indexing it**
    /// — so `DELETE FROM fts WHERE id = ?` cannot seek and scans the whole table. Doing that once
    /// per note makes a rebuild O(n²). Measured before the flag existed: 2 500 notes 2.2 s,
    /// 5 000 notes 10.3 s, 10 000 notes 54.7 s — ~5× per doubling, where linear would be 2×.
    ///
    /// The delete is still correct and still needed for an ordinary write and for the incremental
    /// poll, where it runs for the handful of notes that actually changed rather than for all of
    /// them.
    fn index_object(
        &self,
        obj: &Object,
        path: &Path,
        content: &str,
        fresh: bool,
    ) -> Result<(), StoreError> {
        let id = obj.id.to_string();
        let mtime = mtime_ns(path)?;
        self.db
            .execute(
                "INSERT OR REPLACE INTO objects (id, path, mtime_ns, content)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![id, path.to_string_lossy(), mtime, content],
            )
            .map_err(sql)?;
        if !fresh {
            self.db.execute("DELETE FROM fts WHERE id = ?1", [&id]).map_err(sql)?;
        }
        self.db
            .execute(
                "INSERT INTO fts (id, text) VALUES (?1, ?2)",
                rusqlite::params![id, obj.searchable_text()],
            )
            .map_err(sql)?;
        Ok(())
    }

    /// Refuse to overwrite a note that changed on disk under us — the lost-update
    /// guard, and the reason it lives here rather than in any one command.
    ///
    /// Every writer above this is a read-modify-write over [`Store::get`], which
    /// serves **SQLite, not the file**. So the moment anything else edits a note — a
    /// `git pull`, a merge driver, Vim — the app holds a stale copy, and the next
    /// write rewrites the *whole file* from it. `set_property` is the vicious one: a
    /// board drag rewrites the body too, and the user never thought they were writing
    /// prose. One guard in `put` covers both writers and every future one, and keeps
    /// mtime out of the `Store` trait, where nothing filesystem-shaped belongs.
    ///
    /// The indexed `mtime_ns` is what the file carried when we last read or wrote it.
    /// Still carrying it ⇒ nobody else has touched it and our copy is current. Moved
    /// (or gone) ⇒ it isn't, and the write must not proceed.
    ///
    /// mtime rather than a content hash because a board is re-serialized whole every
    /// 600 ms while drawing: an O(1) stat is affordable on that path and an O(size)
    /// read of a 1 MB scene is not. The price is a filesystem with coarse mtime
    /// granularity, where a save landing in the same tick as a pull would slip
    /// through — nanosecond on ext4/btrfs/xfs, which is what this ships on.
    fn refuse_if_stale(&self, id: Id, path: &Path) -> Result<(), StoreError> {
        let indexed: Option<i64> = self
            .db
            .query_row("SELECT mtime_ns FROM objects WHERE id = ?1", [id.to_string()], |r| r.get(0))
            .optional()
            .map_err(sql)?;
        // Never indexed ⇒ a new note: there is nothing on disk to lose, and `capture`
        // must work. Indexed but now unreadable ⇒ deleted under us, most likely by a
        // pull; re-creating it from our stale copy is the user's call to make, not a
        // debounced auto-save's, so that falls through to a conflict too.
        match indexed {
            None => Ok(()),
            Some(seen) if mtime_ns(path).ok() == Some(seen) => Ok(()),
            Some(_) => Err(StoreError::Conflict(id)),
        }
    }

    /// Which file the index currently believes owns this id, if any. The duplicate-id
    /// guard's question: "is someone else already holding this?"
    fn path_of(&self, id: Id) -> Result<Option<String>, StoreError> {
        self.db
            .query_row("SELECT path FROM objects WHERE id = ?1", [id.to_string()], |r| r.get(0))
            .optional()
            .map_err(sql)
    }

    /// Drop whatever the index holds for one file path, from both tables. Keyed by
    /// path because that is what a filesystem scan knows; the `fts` rows go by the
    /// ids that path currently maps to.
    fn forget_path(&self, path: &str) -> Result<(), StoreError> {
        self.db
            .execute("DELETE FROM fts WHERE id IN (SELECT id FROM objects WHERE path = ?1)", [path])
            .map_err(sql)?;
        self.db.execute("DELETE FROM objects WHERE path = ?1", [path]).map_err(sql)?;
        Ok(())
    }

    /// Parse a stored note and stamp it with this vault's name. **The only place an
    /// object learns where it lives** — `from_file` cannot know, and must not, or the
    /// permission would come from the file's own text.
    fn hydrate(&self, content: &str) -> Result<Object, StoreError> {
        let mut obj = frontmatter::from_file(content).map_err(parse)?;
        obj.vault = self.name.clone();
        Ok(obj)
    }

    fn load_all(&self) -> Result<Vec<Object>, StoreError> {
        let mut stmt = self.db.prepare("SELECT content FROM objects").map_err(sql)?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0)).map_err(sql)?;
        let mut objs = Vec::new();
        for row in rows {
            let content = row.map_err(sql)?;
            objs.push(self.hydrate(&content)?);
        }
        Ok(objs)
    }

    /// Load only the objects whose full text matches an FTS5 expression, joining
    /// back to `objects` for canonical content. Deserializing just the hits (not
    /// all 10k) is what keeps search under the 100 ms budget.
    fn fts_load(&self, match_expr: &str) -> Result<Vec<Object>, StoreError> {
        let mut stmt = self
            .db
            .prepare(
                "SELECT objects.content
                 FROM fts JOIN objects ON objects.id = fts.id
                 WHERE fts MATCH ?1",
            )
            .map_err(sql)?;
        let rows = stmt.query_map([match_expr], |r| r.get::<_, String>(0)).map_err(sql)?;
        let mut objs = Vec::new();
        for row in rows {
            objs.push(self.hydrate(&row.map_err(sql)?)?);
        }
        Ok(objs)
    }
}

impl Store for FileStore {
    fn get(&self, id: Id) -> Result<Option<Object>, StoreError> {
        let content: Option<String> = self
            .db
            .query_row(
                "SELECT content FROM objects WHERE id = ?1",
                [id.to_string()],
                |r| r.get(0),
            )
            .optional()
            .map_err(sql)?;
        match content {
            Some(c) => Ok(Some(self.hydrate(&c)?)),
            None => Ok(None),
        }
    }

    fn put(&mut self, obj: &Object) -> Result<(), StoreError> {
        let content = frontmatter::to_file(obj).map_err(parse)?;
        let path = self.path_for(obj.id);
        self.refuse_if_stale(obj.id, &path)?;
        Self::write_atomic(&path, &content)?;
        // An ordinary write: this id may already be in `fts`, so the delete must run.
        self.index_object(obj, &path, &content, false)?;
        self.written.insert(path);
        Ok(())
    }

    fn delete(&mut self, id: Id) -> Result<(), StoreError> {
        if self.get(id)?.is_none() {
            return Err(StoreError::NotFound(id));
        }
        let path = self.path_for(id);
        if path.exists() {
            fs::remove_file(&path).map_err(io)?;
        }
        // A deletion is a change we made, and git needs it staged as one.
        self.written.insert(path.clone());
        self.db
            .execute("DELETE FROM objects WHERE id = ?1", [id.to_string()])
            .map_err(sql)?;
        self.db
            .execute("DELETE FROM fts WHERE id = ?1", [id.to_string()])
            .map_err(sql)?;
        Ok(())
    }

    /// Full-text is the one predicate FileStore accelerates. With a `Text` in the
    /// filter, resolve it via FTS5 MATCH (loading only the hits) and report `Text` as
    /// **already applied** — the engine must not re-run it, because FTS5 is pinned to
    /// `remove_diacritics 2` here and the engine's substring scan is not, so a second
    /// pass would drop every folded match the index just found. Without `Text`, hand
    /// back everything and the filter whole: the exact MemoryStore path, which is what
    /// keeps the two stores answering identically.
    fn candidates(&self, filter: &Filter) -> Result<(Vec<Object>, Filter), StoreError> {
        match fts_match_expr(filter) {
            Some(expr) => Ok((self.fts_load(&expr)?, strip_text(filter))),
            None => Ok((self.load_all()?, filter.clone())),
        }
    }

    fn reindex(&mut self, mode: Reindex) -> Result<ReindexStats, StoreError> {
        // The whole rebuild runs in ONE transaction: each note is ~3 statements, and
        // per-statement commits are dominated by fsync at 10k notes, so a single
        // commit is the cheap scaling win. On any error we return without
        // committing; the transaction rolls back when the connection drops (open()
        // propagates the error and discards the store).
        let tx = self.db.unchecked_transaction().map_err(sql)?;

        // `Full` drops everything and re-reads every file — the disposable-index
        // escape hatch, and what `open` uses. `Incremental` re-reads only what
        // changed, which is what makes the local poll affordable: without it every
        // beat would re-parse the entire vault.
        //
        // Reconciled by **path**, not id: a note's id lives in its frontmatter, so
        // the two can disagree (a hand-edited `id:`, a file someone copied). Path is
        // what the filesystem gives us to compare against.
        let known: HashMap<String, i64> = match mode {
            Reindex::Full => {
                tx.execute_batch("DELETE FROM objects; DELETE FROM fts;").map_err(sql)?;
                HashMap::new()
            }
            Reindex::Incremental => {
                let mut stmt = self.db.prepare("SELECT path, mtime_ns FROM objects").map_err(sql)?;
                let rows = stmt
                    .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
                    .map_err(sql)?;
                rows.collect::<Result<_, _>>().map_err(sql)?
            }
        };

        let mut scanned = 0usize;
        let mut updated = 0usize;
        let mut skipped: Vec<crate::SkippedNote> = Vec::new();
        // Cloned once rather than read per push: the loop below also takes `&mut self`
        // (`forget_path`), so holding a borrow of `self.name` across it would not compile.
        let vault_name = self.name.clone();
        // A set, not a `Vec`. The deletion sweep below asks "is this indexed path still on
        // disk?" once per indexed note, so a linear membership test made the poll O(n²) —
        // ~10⁸ string comparisons per beat on a 10k-note vault, to discover that
        // nothing had been deleted.
        let mut on_disk = std::collections::HashSet::new();
        for entry in fs::read_dir(&self.notes).map_err(io)? {
            let path = entry.map_err(io)?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            scanned += 1;
            let key = path.to_string_lossy().into_owned();
            on_disk.insert(key.clone());
            // Unchanged since we last read it — the overwhelmingly common case on a
            // poll, and the entire point of doing this incrementally.
            if known.get(&key) == mtime_ns(&path).ok().as_ref() {
                continue;
            }
            // One unreadable note must never stop the vault from opening. The
            // commonest cause is a conflicted merge — git's markers land inside the
            // YAML fence and `from_file` rightly rejects it — and a vault that
            // won't open is one whose conflict UI can never render to fix it.
            // Skip, name it, keep serving everything else.
            let name = || path.file_name().unwrap_or_default().to_string_lossy().into_owned();
            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    skipped.push(crate::SkippedNote {
                        vault: vault_name.clone(),
                        name: name(),
                        path: path.clone(),
                        reason: e.to_string(),
                    });
                    continue;
                }
            };
            match frontmatter::from_file(&content) {
                Ok(obj) => {
                    // **Two files claiming one id is a skip, not a race to win.**
                    //
                    // `objects.id` is the primary key and `index_object` is INSERT OR
                    // REPLACE, so a duplicated `id:` (someone copied a note file rather
                    // than making one) collapses to a single row whose `path` alternates.
                    // Every poll then finds whichever path the row is *not* currently
                    // pointing at, re-indexes it, reports `updated: 1` — and the UI
                    // refreshes, forever, at the beat interval. `ReindexStats::removed`
                    // documents that exact trap for a note that never parses; this is the
                    // same trap by another route, and it was not covered.
                    //
                    // So: first path wins, the other is named. Serving one file's content
                    // under another's id is worse than serving neither, and "loud and
                    // recoverable" is the discipline the unreadable-note skip already sets.
                    if let Some(other) = self.path_of(obj.id)? {
                        if other != key && Path::new(&other).exists() {
                            skipped.push(crate::SkippedNote {
                                vault: vault_name.clone(),
                                name: name(),
                                path: path.clone(),
                                reason: format!(
                                    "duplicate id {} — already held by {}. Give one of them a \
                                     fresh id; until then only the first is indexed.",
                                    obj.id,
                                    Path::new(&other)
                                        .file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                ),
                            });
                            continue;
                        }
                    }
                    // Drop whatever this path held before. Normally that is the same
                    // object and `index_object`'s REPLACE would cover it — but if the
                    // file's `id:` changed under us, the old row would otherwise
                    // linger and serve this note's content twice, under two ids.
                    // Nothing to forget during a `Full` rebuild: both tables were emptied
                    // before the loop, so this would be three table scans to delete rows that
                    // cannot exist. It stays for the incremental path, where the file's `id:`
                    // may genuinely have changed under us and the old row must go.
                    if mode == Reindex::Incremental {
                        self.forget_path(&key)?;
                    }
                    self.index_object(&obj, &path, &content, mode == Reindex::Full)?;
                    updated += 1;
                }
                Err(e) => {
                    // It parsed last time and doesn't now (a pull just landed
                    // markers in it): drop the stale row rather than keep serving a
                    // version of the note that is no longer on disk.
                    self.forget_path(&key)?;
                    skipped.push(crate::SkippedNote {
                        vault: vault_name.clone(),
                        name: name(),
                        path: path.clone(),
                        reason: e.to_string(),
                    });
                }
            }
        }
        // Files that vanished — deleted in a pull, or by `rm`. A full rebuild has
        // already dropped them by construction.
        let mut removed = 0usize;
        if mode == Reindex::Incremental {
            for gone in known.keys().filter(|k| !on_disk.contains(k.as_str())) {
                self.forget_path(gone)?;
                removed += 1;
            }
        }
        tx.commit().map_err(sql)?;
        // Remember it too, so a caller holding the store can report what vanished
        // without having to reindex again just to ask.
        self.skipped = skipped.clone();
        Ok(ReindexStats { scanned, updated, removed, skipped })
    }
}

/// Build the FTS5 MATCH expression for a query, or `None` when the pure path
/// should handle it: no `Text` predicate at all, or a `Text` needle that yields
/// no FTS tokens (e.g. punctuation only) — which FTS can't express but a
/// substring scan can. Every token becomes a prefix term (`"tok"*`) so the
/// search box matches as you type; terms AND together, matching FTS5's default.
///
/// `None` is only returned when it is safe: either there is nothing to strip, or
/// *some* `Text` needle is inexpressible, in which case the whole query (Text
/// included) falls back to the substring engine rather than dropping a filter.
fn fts_match_expr(filter: &Filter) -> Option<String> {
    let needles: Vec<&String> = filter
        .all
        .iter()
        .filter_map(|p| match p {
            Predicate::Text(s) => Some(s),
            _ => None,
        })
        .collect();
    if needles.is_empty() {
        return None;
    }
    let mut terms = Vec::new();
    for needle in needles {
        let tokens = fts_tokens(needle);
        if tokens.is_empty() {
            return None; // inexpressible in FTS — let the substring engine handle it
        }
        for t in tokens {
            terms.push(format!("\"{t}\"*"));
        }
    }
    Some(terms.join(" "))
}

/// Split a needle into FTS-safe tokens: alphanumeric runs, lowercased. Because
/// tokens are alphanumeric-only, double-quoting them in the MATCH expression
/// cannot inject FTS operators.
fn fts_tokens(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// Drop `Text` predicates from a query — the FTS MATCH already applied them, so
/// the pure engine only evaluates the remaining (structured) predicates.
/// The filter minus what FTS5 already answered — see [`Store::candidates`].
fn strip_text(filter: &Filter) -> Filter {
    let mut f = filter.clone();
    f.all.retain(|p| !matches!(p, Predicate::Text(_)));
    f
}

fn mtime_ns(path: &Path) -> Result<i64, StoreError> {
    let modified = fs::metadata(path).map_err(io)?.modified().map_err(io)?;
    Ok(modified
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0))
}

fn io(e: std::io::Error) -> StoreError {
    StoreError::Io(e.to_string())
}
fn sql(e: rusqlite::Error) -> StoreError {
    StoreError::Io(format!("sqlite: {e}"))
}
fn parse(e: frontmatter::ParseError) -> StoreError {
    StoreError::Parse(e.to_string())
}
