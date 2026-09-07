//! **The one command surface.** Every command the app can perform, dispatched by name over
//! the open vaults — with the lock discipline that makes it safe from more than one thread.
//!
//! This is the door, and there is deliberately only one of it. Before this module the
//! `match` below lived in `fm-serve`'s `api()`, which made HTTP the *only* way to reach a
//! command: a second frontend could not call [`dispatch`], only re-implement it. `fm-cli`
//! already demonstrates the cost of that — it re-implements the same flows against `fm-core`
//! rather than calling [`crate::commands`], so the surface has forked once already. A third
//! fork was the price of the next frontend, and every one of them would have had to
//! independently rediscover the lock discipline documented on [`Vaults`].
//!
//! So a transport now owns **only** its framing: parse a request into `(cmd, args, body)`,
//! call [`dispatch`], turn [`Output`] back into whatever it speaks. `fm-serve` is that shell
//! for HTTP. Nothing about this module is HTTP-shaped — no URLs, no percent-decoding, no
//! status codes — which is the test of whether the extraction was real.
//!
//! The single genuinely platform-bound arm, `open_external`, is a seam rather than a fork:
//! see [`Host`].

use crate::commands;
use crate::scope::Scope;
use crate::vaults::{self, VaultConfig};
use fm_core::{backup, git, vcs, ColdStart, MultiStore, Reindex, Scoped, Store};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

/// The open vaults and the list describing them, **behind one lock**.
///
/// One and not two, for a reason that is already latent in [`dispatch`]: its arms take these
/// in *opposite orders* — `commit`/`push` lock the store and then resolve the vault, while
/// `ingest` resolves the vault and then locks the store. Two mutexes would make that a
/// textbook AB/BA deadlock. One deletes the question, and closes the window where the list
/// holds a vault the store doesn't.
pub struct Vaults {
    /// **Every** vault, unconditionally. Reached through [`Vaults::store`], which narrows it to
    /// what the caller is an audience for; the field is private and named `all` so that reaching
    /// past that narrowing has to be deliberate and reads as such at the call site.
    all: MultiStore,
    /// Every vault, in configured order. **The first is the default**: a note that names
    /// no vault (every fresh capture) lands there, so it should be the personal one. A
    /// single-vault install is just a list of one, which is why nothing below has a
    /// "multi" special case. **Empty is the first run**, not an error.
    ///
    /// Parallel to `store`: [`Vaults::add`] is the only thing that grows either, and it
    /// pushes both.
    list: Vec<VaultConfig>,
}

/// Everything [`dispatch`] needs that outlives one command: the vaults, and the two facts
/// about the vault list *file* that `check_path`/`create_vault` answer with.
///
/// The lock lives **inside** here rather than around the whole thing on purpose. Taking a
/// `&mut Vaults` would read as the simpler signature, but it would hold the lock for the
/// entire command — and **several arms exist precisely to *drop* it before doing slow I/O**
/// (`asset_status`, `resolve_asset`, `open_external`, `backup`, `backup_status`,
/// `ingest_unlocked`, `activity`, and more). Deliberately not a count: this sentence said
/// "five arms" and named five, `activity` turned out to be a sixth that did **not** drop the
/// lock, and the confident tally is part of why nobody looked — a list that names five reads as
/// exhaustive. See `known-issues.md`'s trap, *a count in a comment is a claim, and it rots*.
/// `backup_status` is the sharpest: it shells out to `git ls-remote` per vault, and holding
/// the lock across that network round trip would stall every `ping` behind it. So each
/// arm still takes the lock for exactly as long as it needs it, exactly as it did when this
/// lived in the server.
///
/// One mutex, and it is exclusive — there is no reader/writer split here, so "holds the
/// lock" always means "nothing else may touch a vault meanwhile", read or write.
pub struct App {
    vaults: Mutex<Vaults>,
    /// The vault list file we would write, or `None` when this machine has no config dir
    /// at all — in which case a vault cannot be persisted and `create_vault` says so
    /// *before* it creates any directories.
    config: Option<PathBuf>,
    /// Whether we understood the vault list on disk. False means never write it:
    /// overwriting a hand-edited file we could not parse is the loss `vaults::load`'s
    /// malformed-JSON warning exists to shout about.
    config_writable: bool,
    /// **How many times the vaults have moved, ever.** Monotonic, per-process, and the thing
    /// `ping` compares a client's `since` against.
    ///
    /// It exists because the old answer — "did this reindex find drift?" — is a fact that can
    /// only be told **once**. A reindex writes the fresh mtimes back, so the first client to
    /// ask consumes the news and every other client is told "nothing changed", forever. With
    /// one browser tab that was invisible; with a tab and a tablet it is the common case.
    ///
    /// It also closes a hole the drift check never covered at all. `FileStore::put` indexes
    /// the file it just wrote, mtime included (`file.rs::index_object`), so after a save
    /// through `dispatch` the index and the disk **agree** — an incremental reindex finds
    /// nothing, and a note written by one client was therefore invisible to every other
    /// client rather than merely late. The poll only ever saw *out-of-band* edits (a `git
    /// pull`, the merge driver, Vim), which was sufficient for exactly as long as there was
    /// only ever one client.
    ///
    /// So it is bumped from both sides: by a reindex that found drift, and by any command
    /// that is not on [`READ_ONLY`]. Not a lock, because it is only ever read and incremented
    /// — a client that misses one increment sees the next.
    generation: AtomicU64,
}

/// What a transport must supply that the command library cannot: the one operation whose
/// implementation is a property of the *platform*, not of the vault.
///
/// Exactly one command needs this — `open_external`, which hands a blob to whatever the OS
/// thinks owns it. On a desktop that is `xdg-open`/`open`/`start`; the same call has no
/// meaning inside an Android app, which routes it through a content provider instead. A
/// `#[cfg(target_os)]` ladder in here would compile a wrong answer for every platform not
/// yet listed; a trait makes the shell say what it can actually do, and makes the arm
/// testable with a fake.
pub trait Host {
    /// Hand a file to whatever this platform thinks owns it.
    fn open_external(&self, path: &Path) -> Result<(), String>;
}

/// What a command answered with, and *what kind of thing it is* — so a transport can frame
/// the reply without a table of command names. (The server used to special-case the string
/// `"resolve_asset"` when choosing a Content-Type; that is the command surface leaking into
/// the transport, and it would have leaked into the next one too.)
pub enum Output {
    /// A JSON document. Empty for the several commands that answer with no content —
    /// `set_property`, `update_body`, `delete`, `open_external`, `backup`, `set_git_remote`,
    /// `uncopy_note` — which report success by not failing.
    Json(Vec<u8>),
    /// Raw bytes with no structure of their own: blob content, for the transport to frame
    /// however it frames binary.
    Bytes(Vec<u8>),
}

impl Output {
    /// The bytes, whatever kind they were.
    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            Output::Json(b) | Output::Bytes(b) => b,
        }
    }
}

/// **`App` must stay shareable across threads, and this is the only place CI can check it.**
///
/// Every transport already relies on it — `fm-serve` is thread-per-connection, and the mobile
/// shell's commands are `#[tauri::command(async)]`, so `tauri`'s `.manage(Arc<App>)` and its
/// spawned futures both require `Send + Sync + 'static`. But the mobile crate is excluded from
/// the workspace, so if a field here were swapped for something thread-hostile (an `Rc`, a
/// `RefCell`, a raw pointer) `pixi run ci` would stay green and the breakage would appear only in
/// an Android build nobody runs on the change that caused it.
///
/// A `const` assertion costs nothing at runtime and fails at compile time, in `cargo test
/// --workspace`, naming exactly what broke.
const _: () = {
    const fn assert_shareable<T: Send + Sync + 'static>() {}
    assert_shareable::<App>();
};

impl App {
    /// Open every configured vault and build the surface over them.
    ///
    /// Returns the notes that could not be read alongside it. A vault opens even when a
    /// note is unreadable — a conflicted merge is the usual cause, and refusing to start
    /// would take away the very app you need to fix it — but those notes are then absent
    /// from every view, so the caller **must** say which ones. Silently serving an
    /// incomplete vault is the one outcome worse than not starting at all, which is why
    /// this hands them back rather than logging them somewhere a frontend cannot see.
    pub fn load() -> Result<(Self, Vec<String>), String> {
        Self::load_with(ColdStart::Rebuild)
    }

    /// [`App::load`], with the shell's claim about how much of the index it can trust.
    ///
    /// **Only a shell can answer this**, which is why it is a parameter and not a `cfg!` in
    /// `fm-core`. See [`ColdStart`]: trusting the persisted index is safe exactly where no tool
    /// can rewrite a note file while preserving its mtime. The Android shell knows it is on a
    /// platform with no shell, no restic and no `rsync`; `fm-serve` knows the opposite. Neither
    /// fact belongs to the store, and a `#[cfg(target_os)]` inside the library would compile a
    /// wrong answer for every platform not yet listed — the same reasoning that made
    /// `open_external` a trait rather than a ladder.
    pub fn load_with(cold: ColdStart) -> Result<(Self, Vec<String>), String> {
        let vaults::VaultList {
            vaults,
            path: config,
            writable: config_writable,
        } = vaults::load();
        let mut store = MultiStore::open_with(
            &vaults
                .iter()
                .map(|v| (v.name.clone(), v.path.clone()))
                .collect::<Vec<_>>(),
            cold,
        )
        .map_err(|e| format!("open vaults: {e}"))?;
        // **Rebuild the write-record from the filesystem, once, here.**
        //
        // `commit_all` stages what this app remembers writing, and that memory dies with the process.
        // Android kills backgrounded apps constantly, so almost every relaunch orphaned the previous
        // run's notes — permanently unstageable, and silent until the count was surfaced (147 of them
        // on the owner's phone, 2026-07-31). Open is exactly where the memory was lost, so it is where
        // it is rebuilt: the next ordinary commit then records them, with no user action and no new
        // staging semantics.
        //
        // **Only at open, never on a timer**, so the deliberate property that a note being edited by
        // hand is not swept mid-sentence still holds for the whole session. And only paths that are
        // ours *by construction* — see `adoptable`.
        for cfg in &vaults {
            let adopted = adoptable(&cfg.path);
            if !adopted.is_empty() {
                let n = store.seed_written(&cfg.name, adopted);
                // Said out loud: a startup that quietly adopts files is the kind of thing nobody can
                // account for later. The UI states it too, from `unrecorded`.
                eprintln!(
                    "note: adopting {n} unrecorded note(s) in '{}' for the next commit",
                    cfg.name
                );
            }
        }
        // **Sweep abandoned chunked uploads, here, for the same reason as the block above.**
        //
        // A process killed mid-upload leaves a session directory behind (`fm_core::chunked`), and
        // on a phone that is not an edge case — Android kills backgrounded apps whenever it
        // likes, which is precisely why the write-record above has to be rebuilt at all. Open is
        // the one moment nothing is in flight, so it is where the leftovers go.
        //
        // Age-based, never "delete everything": two uploads can be live across a restart on a
        // device with a paired tablet, and collecting one of those would turn a survivable
        // interruption into a failed upload. Best-effort — a session that will not delete is disk
        // to reclaim, not a reason to refuse to start.
        for cfg in &vaults {
            let swept = fm_core::chunked::sweep(&cfg.path);
            if swept > 0 {
                eprintln!("note: swept {swept} abandoned upload(s) in '{}'", cfg.name);
            }
        }
        // Flattened to strings here on purpose: this is the startup log line, which wants
        // one readable sentence per note. The structured form is what rides the heartbeat.
        let skipped = store
            .skipped()
            .iter()
            .map(|s| format!("{}: {}: {}", s.vault, s.name, s.reason))
            .collect();
        Ok((Self::new(store, vaults, config, config_writable), skipped))
    }

    /// The pieces, already assembled — for tests, and for a shell that sources its vault
    /// list from somewhere other than this machine's config file.
    pub fn new(
        store: MultiStore,
        list: Vec<VaultConfig>,
        config: Option<PathBuf>,
        config_writable: bool,
    ) -> Self {
        App {
            vaults: Mutex::new(Vaults { all: store, list }),
            config,
            config_writable,
            generation: AtomicU64::new(0),
        }
    }

    /// Record that the vaults moved, and return the new generation.
    fn bump(&self) -> u64 {
        self.generation.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// An owned snapshot of the configured vaults — what a shell prints at startup.
    pub fn configs(&self) -> Result<Vec<VaultConfig>, String> {
        Ok(self.lock()?.configs())
    }

    /// The vault guard, **recovering from a poisoned mutex rather than propagating it**.
    ///
    /// A `PoisonError` means some earlier call panicked while holding this guard. Propagating it
    /// bricked the whole process: every command afterwards answered with the `Display` of a
    /// `PoisonError`, which is not a sentence anyone can act on, and there is no path back except
    /// restarting the app. That is not hypothetical — `paper.rs` records one multi-byte character
    /// in one PDF doing exactly this. The agent thread and the webview both dispatch, so a panic
    /// in either used to poison both.
    ///
    /// **Is the poisoning load-bearing? No, and the argument is worth keeping.** Poisoning guards
    /// against reading torn state, and there is one torn state to worry about: `Vaults::add` pushes
    /// to `all` and `list` in turn, and its own doc says that is "so `store` and `list` cannot
    /// disagree". But both halves are *reconstructions of on-disk truth* — `list` mirrors
    /// `vaults.json`, which `import_into_new_vault` deliberately writes **before** memory ("JSON
    /// before memory, so a failed write never leaves a vault that vanishes on restart"), and `all`
    /// is an index over files that `reindex` rebuilds. So the worst case here is a transient
    /// in-memory disagreement that a restart fixes, weighed against a process that answers nothing
    /// until it is restarted anyway.
    ///
    /// **Recovery must be loud, or a panic becomes invisible** — which would be a worse bug than
    /// the one this fixes. Logged once per recovery, not once per call.
    fn lock(&self) -> Result<MutexGuard<'_, Vaults>, String> {
        match self.vaults.lock() {
            Ok(g) => Ok(g),
            Err(poisoned) => {
                eprintln!(
                    "warning: recovering the vault lock after an earlier panic — some command \
                     failed part-way through. Your notes are files and are not at risk, but if \
                     anything looks wrong, restart the app."
                );
                Ok(poisoned.into_inner())
            }
        }
    }

    /// Panic while holding the vault guard, so a test can prove recovery works.
    ///
    /// `catch_unwind` keeps the panic inside this call — the point is to leave the mutex poisoned,
    /// not to fail the test that arms it.
    #[cfg(test)]
    pub(crate) fn poison_for_test(&self) {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _g = self.vaults.lock().expect("not yet poisoned");
            panic!("deliberately poisoning the vault mutex");
        }));
    }
}

/// Perform one command.
///
/// `args` carries the named arguments (the transport's job to assemble — from a JSON body,
/// a query string, an IPC payload, whatever it speaks). `body` is the raw request payload,
/// which only `ingest` reads: there the bytes *are* the file, so they cannot also be the
/// arguments, which is why `ingest`'s name and vault arrive out-of-band.
///
/// An unknown command is an `Err`, not a panic — a transport must be able to answer a
/// malformed request without taking the process down.
///
/// Every command that is not in [`READ_ONLY`] bumps [`App::generation`] when it succeeds, so
/// that other clients learn the vaults moved. That is done here, once, rather than in each
/// arm: fifty-odd arms each remembering to call a function is fifty-odd chances to forget, and
/// the one that forgets fails *silently* — a note saved on the tablet that simply never appears
/// on the desktop.
/// The machine's own caller — every vault, as it has always been. `fm-cli`, the phone, the study
/// agent and anything on loopback come through here and are unchanged by the existence of scopes.
pub fn dispatch(
    cmd: &str,
    args: &Value,
    body: &[u8],
    app: &App,
    host: &dyn Host,
) -> Result<Output, String> {
    dispatch_as(cmd, args, body, app, host, &Scope::All)
}

/// The same, for a caller entitled to only some of the audiences — a paired device.
///
/// A separate entry point rather than an extra parameter on [`dispatch`], because the
/// unrestricted case is the overwhelmingly common one and every existing shell already spells
/// it: making them all pass `Scope::All` would be fifty edits that say nothing, and a default
/// argument nobody reads is how the wrong one gets passed.
pub fn dispatch_as(
    cmd: &str,
    args: &Value,
    body: &[u8],
    app: &App,
    host: &dyn Host,
    scope: &Scope,
) -> Result<Output, String> {
    let out = dispatch_inner(cmd, args, body, app, host, scope);
    // **Only on success**, so a rejected write (a stale `base`, an unknown vault) does not
    // send every other client off to re-query for a change that never landed.
    if out.is_ok() && !READ_ONLY.contains(&cmd) {
        app.bump();
    }
    out
}

/// The repository name behind a vault's remote, if it has one: `…/formicarium-vault.git` →
/// `formicarium-vault`. `None` when there is no remote, or nothing usable in the URL.
///
/// A local `git config` read — no network — so the vault list can carry it without touching the
/// `git ls-remote` path that makes `backup_status` the slow call.
fn remote_label(path: &std::path::Path) -> Option<String> {
    let url = vcs::remote(path).ok().flatten()?;
    let last = url.trim_end_matches('/').rsplit(['/', ':']).next()?;
    let name = last.strip_suffix(".git").unwrap_or(last).trim();
    (!name.is_empty()).then(|| name.to_string())
}

/// One plain sentence per conflict kind — what the two sides actually did.
///
/// Product wording, so it lives here at the command surface rather than in `fm-core`: the core knows
/// git's seven codes and nothing about explaining them to someone holding a phone.
fn describe_conflict(kind: git::ConflictKind) -> String {
    use git::ConflictKind::*;
    match kind {
        BothModified => "Both sides edited this note; both versions are marked in the text.",
        BothAdded => "Both sides created a different note here; both versions are marked in the text.",
        DeletedByUs => "Deleted here, edited on the other device. There is nothing to merge \u{2014} pick a side.",
        DeletedByThem => "Edited here, deleted on the other device. There is nothing to merge \u{2014} pick a side.",
        BothDeleted => "Deleted on both sides, differently. There is nothing to keep.",
        AddedByUs => "Created here, and the other side does not have it.",
        AddedByThem => "Created on the other device, and this side does not have it.",
    }
    .to_string()
}

/// When the note says it was created, from its own frontmatter — the stamp shown beside an
/// unrecorded note, so "written 7 days ago and never committed" is legible at a glance.
fn created_of(full: &std::path::Path) -> Option<String> {
    let text = std::fs::read_to_string(full).ok()?;
    let obj = fm_core::frontmatter::from_file(&text).ok()?;
    Some(crate::dto::stamp(obj.created))
}

/// Which code path wrote this note, from the markers the note itself carries.
///
/// Reuses `thread::is_message`/`is_proposal` — the same predicates the query layer filters the planning
/// views with — so this cannot drift from what the rest of the app means by "a message". A title says
/// what a note *is about*; this says **who made it**, which is the question when 142 of them appear in
/// one minute.
fn role_of(full: &std::path::Path) -> String {
    let Ok(text) = std::fs::read_to_string(full) else {
        return "deleted".into();
    };
    match fm_core::frontmatter::from_file(&text) {
        Ok(o) if crate::thread::is_proposal(&o) => "proposal".into(),
        Ok(o) if crate::thread::is_message(&o) => "message".into(),
        Ok(_) => "note".into(),
        Err(_) => "unreadable".into(),
    }
}

/// A note's body, normalised, as the key for counting duplicates. The *body* and not the whole file:
/// two copies of one message differ in `id` and `created`, so hashing the file would report every
/// duplicate as unique — which is exactly the mistake that would hide the finding.
fn body_key(full: &std::path::Path) -> Option<String> {
    let text = std::fs::read_to_string(full).ok()?;
    let obj = fm_core::frontmatter::from_file(&text).ok()?;
    let body = obj.body.trim();
    (!body.is_empty()).then(|| fm_core::blob::sha256_hex(body.as_bytes()))
}

/// Notes git does not have that this app may safely claim as its own writes.
///
/// **The whole safety argument is the naming scheme.** `add -A` was rejected because a vault may also
/// be a project repo, and staging everything would make this app a second author of someone's index.
/// A file at `<notes dir>/<ULID>.md` is not ambiguous: that name is what `FileStore` writes and
/// nothing else produces it, so adopting exactly those cannot touch a hand-written file. A conflicted
/// path is already excluded by `unrecorded` itself.
///
/// Empty when there is no git, no repo, or nothing outstanding — the common case, costing one
/// `git status` per vault at open and nothing after.
fn adoptable(vault: &std::path::Path) -> Vec<std::path::PathBuf> {
    if !vcs::available() {
        return Vec::new();
    }
    let notes_rel = notes_rel_of(vault);
    vcs::unrecorded(vault, &notes_rel)
        .unwrap_or_default()
        .into_iter()
        .filter(|u| {
            std::path::Path::new(&u.path)
                .file_stem()
                .and_then(|s| s.to_str())
                .is_some_and(|stem| stem.parse::<fm_model::Id>().is_ok())
        })
        .map(|u| vault.join(u.path))
        .collect()
}

/// A note's title, or its first non-empty body line — something a human recognises, for a list that
/// would otherwise be ULIDs. `None` when the file is not there (a deleted note) or will not parse.
///
/// Reads one small file per row, and only for the bounded detail list — never for the counts.
fn title_of(full: &std::path::Path) -> Option<String> {
    let text = std::fs::read_to_string(full).ok()?;
    // The vault's own parser, not a second one — `frontmatter::from_file` is what `FileStore` reads
    // notes with, so a title shown here is the title the app would show anywhere else.
    //
    // **Fall back to the parsed BODY, never to the raw file.** The first version scanned `text`, so an
    // untitled note reported its first frontmatter key — every one of them came back titled
    // `schema: 1`. Seen on the owner's phone within an hour of shipping it, where it read as "142
    // malformed notes" and nearly sent the diagnosis down a wrong path. A diagnostic that lies is
    // worse than no diagnostic, which is the whole argument for this panel existing.
    if let Ok(obj) = fm_core::frontmatter::from_file(&text) {
        if let Some(t) = obj.title.as_ref().filter(|t| !t.trim().is_empty()) {
            return Some(t.trim().to_string());
        }
        return obj
            .body
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|l| l.chars().take(72).collect());
    }
    // Unparseable: there is no body to speak of, so say nothing rather than quoting YAML at the user.
    None
}

/// The vault-relative notes directory (`notes` unless `vault.json` says otherwise).
///
/// Asked of the descriptor rather than hardcoded: a vault may keep its notes in `docs/`, and
/// hardcoding `notes/` is precisely the bug that had `verify` pass a vault it never opened.
fn notes_rel_of(root: &std::path::Path) -> String {
    fm_core::descriptor::Descriptor::read(root)
        .ok()
        .and_then(|d| {
            d.notes_dir(root)
                .strip_prefix(root)
                .ok()
                .map(|p| p.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "notes".to_string())
}

/// The commands that cannot move a vault, and so must **not** bump the generation.
///
/// **An allowlist, deliberately, and this direction is the safe one.** A new command left off
/// this list bumps when it perhaps need not: every client re-runs its query once, which costs a
/// SQLite read and is invisible. A new *writing* command left off a mutating list would instead
/// be silently invisible to every other client — the exact bug this counter exists to fix. So
/// the failure mode of forgetting is "slightly chatty", never "silently stale".
///
/// `ping` is here because it is the poll itself: bumping in the poll would make every client
/// see a change on every beat, forever. It bumps from *inside* its arm instead, and only when
/// the reindex actually found drift.
///
/// `open_external`/`open_skipped` hand a file to another program and change nothing here; if
/// that program then edits the note, the drift check is what catches it.
const READ_ONLY: &[&str] = &[
    "board",
    "gallery",
    "agenda",
    "get",
    "search",
    "recent",
    "proposals",
    "backlinks",
    "templates",
    "conflicts",
    // A read like `conflicts`: it asks git what is not recorded and changes nothing.
    "unrecorded",
    // Also a read — it groups notes by body and names what pruning *would* remove.
    "duplicates",
    "activity",
    "stale",
    "thread",
    "thread_roots",
    "discussions",
    "proposal_diff",
    "proposal_content",
    "proposal_for",
    "asset_status",
    "resolve_asset",
    // A `store.get` and a `format!`. Absent from this list, copying a citation bumped the
    // generation and told every connected client — desktop, phone, paired tablet — that the
    // vault had changed, so each refetched its whole workspace for nothing.
    "paper_bibtex",
    "backup_status",
    // A `restic snapshots` and nothing else. It changes no vault, so bumping the generation for
    // it would tell every connected client the vault moved because somebody opened a panel —
    // exactly the `paper_bibtex` lesson.
    "backup_latest",
    "ping",
    "read_skipped",
    "list_vaults",
    "list_views",
    "run_view",
    // Reading a theme changes nothing. Writing or deleting one deliberately stays *off* this list:
    // the generation bump is what makes another machine (or the other pane) notice a theme edit,
    // for free, without a timer of its own.
    "list_themes",
    "read_theme",
    "check_path",
    // Asked on every keystroke while someone types a folder, exactly like `check_path`. Off this
    // list it would bump the generation each time and tell every connected client the vault had
    // changed — the `paper_bibtex` lesson, on a faster trigger.
    "check_import",
    "config",
    "probe_remote",
    "git_auth",
    "copy_status",
    "open_external",
    "open_skipped",
];

fn dispatch_inner(
    cmd: &str,
    args: &Value,
    body: &[u8],
    app: &App,
    host: &dyn Host,
    scope: &Scope,
) -> Result<Output, String> {
    let s = |k: &str| {
        args.get(k)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    let lock = || app.lock();

    match cmd {
        "board" => json(commands::board(&lock()?.store(scope), &s("groupBy")).map_err(err)?),
        "gallery" => json(commands::gallery(&lock()?.store(scope)).map_err(err)?),
        "agenda" => json(commands::agenda(&lock()?.store(scope)).map_err(err)?),
        "get" => json(commands::get(&lock()?.store(scope), &s("id")).map_err(err)?),
        "search" => json(commands::search(&lock()?.store(scope), &s("query")).map_err(err)?),
        "recent" => json(commands::recent(&lock()?.store(scope)).map_err(err)?),
        // The Collaboration surface's feed: every open proposal (a note carrying
        // `proposes: branch:<name>`), across vaults. A store query like `recent`, not a per-vault
        // git read — it lists proposal *notes*; whether each branch is still open is a git
        // question answered later, when the write half and the diff land.
        "proposals" => json(commands::proposals(&lock()?.store(scope)).map_err(err)?),
        // "What links here" — notes whose body references this note (a `note:` mention or an embed).
        // A store scan like `recent`, not a git read; no reverse index.
        "backlinks" => json(commands::backlinks(&lock()?.store(scope), &s("id")).map_err(err)?),
        "templates" => json(commands::templates(&lock()?.store(scope)).map_err(err)?),
        // **Conflicts, with their kind** — because "open it and keep the text you want" is true of
        // exactly one kind. Git is asked first and is authoritative (it is what actually blocks every
        // commit in the vault, and it is the only thing that knows *how* the two sides disagreed);
        // the body-marker scan is then unioned in for notes whose index entry git has settled but
        // whose text still carries markers. See `dto::ConflictInfo`.
        "conflicts" => {
            let mut g = lock()?;
            let scanned = commands::conflicts(&g.store(scope)).map_err(err)?;
            let configs = g.configs();
            let mut out: Vec<crate::dto::ConflictInfo> = Vec::new();
            let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
            for cfg in &configs {
                for c in vcs::conflicted(&cfg.path).unwrap_or_default() {
                    let Some(stem) = std::path::Path::new(&c.path)
                        .file_stem()
                        .and_then(|s| s.to_str())
                    else {
                        continue;
                    };
                    if !seen.insert(stem.to_string()) {
                        continue;
                    }
                    // Prefer the indexed note. A delete/modify conflict may have no indexed note at
                    // all (this side deleted it), so fall back to a stub — the point is that every
                    // note the "waiting on you" warning names is *findable and actionable* here,
                    // never a dangling toast, and never advice the user cannot follow.
                    let note = match stem
                        .parse::<fm_model::Id>()
                        .ok()
                        .and_then(|id| g.store(scope).get(id).ok().flatten())
                    {
                        Some(obj) => crate::dto::ObjectMeta::from(&obj),
                        None => crate::dto::ObjectMeta {
                            id: stem.to_string(),
                            kind: "note".into(),
                            title: Some("a note deleted here and edited elsewhere".into()),
                            preview: format!("Unmerged in \u{201c}{}\u{201d}.", cfg.name),
                            // Only the feed carries one, and a conflict placeholder never reaches it.
                            excerpt: None,
                            status: None,
                            due: None,
                            start: None,
                            hard: false,
                            created: String::new(),
                            updated: String::new(),
                            tags: Vec::new(),
                            assets: Vec::new(),
                            props: Default::default(),
                            vault: cfg.name.clone(),
                        },
                    };
                    out.push(crate::dto::ConflictInfo {
                        note,
                        path: c.path.clone(),
                        vault: cfg.name.clone(),
                        code: c.kind.code().to_string(),
                        what: describe_conflict(c.kind),
                        has_markers: c.kind.has_markers(),
                    });
                }
            }
            for m in scanned {
                if seen.contains(&m.id) {
                    continue;
                }
                let vault = m.vault.clone();
                out.push(crate::dto::ConflictInfo {
                    note: m,
                    path: String::new(),
                    vault,
                    code: String::new(),
                    what: "Both versions are marked in the text.".into(),
                    has_markers: true,
                });
            }
            json(out)
        }
        // **Resolve a conflict that cannot be resolved by editing.** For a delete/modify there is no
        // text to fix: one side has no file, so the only answers are which side stands. Git's own
        // verbs are `add` (keep the file) and `rm` (keep the deletion), and neither had a UI — which
        // for a user who only ever sees the UI meant the conflict was unresolvable, and the vault
        // stayed frozen. Finishing the merge when this was the last one is part of the operation:
        // `commit_all` would otherwise leave MERGE_HEAD standing, and MERGE_HEAD is what makes it
        // refuse.
        "resolve_conflict" => {
            let keep = match s("keep").as_str() {
                "theirs" => git::Keep::Theirs,
                "mine" => git::Keep::Mine,
                // "I reconciled both versions in the editor" — the answer for a marker conflict,
                // where neither side alone is right. Refused by the backend while markers remain.
                "edited" => git::Keep::Edited,
                other => {
                    return Err(format!(
                        "keep must be \"theirs\", \"mine\" or \"edited\", not \"{other}\""
                    ))
                }
            };
            let path = s("path");
            if path.is_empty() {
                return Err("resolve_conflict needs the conflicted path".into());
            }
            let mut g = lock()?;
            let cfg = g.config(scope, &s("vault"))?;
            vcs::resolve_conflict(&cfg.path, &path, keep).map_err(err)?;
            // The resolution wrote (or removed) a file behind the index's back, exactly as `pull`
            // does — re-read now rather than leave the user looking at what was there before.
            g.store(scope).reindex(Reindex::Incremental).map_err(err)?;
            // No explicit bump: `resolve_conflict` is deliberately absent from READ_ONLY, so the
            // wrapper bumps for us once this returns Ok — and only then.
            json(serde_json::json!({ "resolved": path }))
        }
        // **What the app forgot it wrote, and in which way.** Per vault: notes on disk that git does
        // not have, split by kind, with a bounded sample of detail.
        //
        // The count alone was not a diagnosis. On the owner's phone it said 146, and that could have
        // meant 146 notes existing nowhere else (urgent — app-private storage is erased by an
        // uninstall) or 146 notes something was needlessly rewriting (a different bug entirely). The
        // device knew which and had no way to say it, and the phone has no other channel: no readable
        // logcat, no console, no shell. So the answer carries the kinds.
        // Identical notes, grouped. A read: it changes nothing and names what pruning would remove.
        "duplicates" => {
            let mut g = lock()?;
            json(commands::duplicates(&g.store(scope)).map_err(err)?)
        }
        // **Remove the extra copies — but only ones git already has.**
        //
        // Deleting an *untracked* note is unrecoverable: there is no commit to restore it from, and
        // this codebase does not destroy user data on any path, including the ones the user asked for.
        // A tracked note is one `git checkout` away. So the order is forced: record first, prune second,
        // and this arm refuses rather than doing the dangerous half. That refusal is the safety
        // property — not a warning in a doc comment.
        //
        // Keeps the oldest copy of every family, always. The most a mistake can cost is a commit that
        // says which ids went, which is exactly what makes it undoable.
        "prune_duplicates" => {
            let mut g = lock()?;
            let cfg = g.config(scope, &s("vault"))?;
            let notes_rel = notes_rel_of(&cfg.path);
            // Everything in this vault that git does not have, by id.
            let outstanding: std::collections::HashSet<String> =
                vcs::unrecorded(&cfg.path, &notes_rel)
                    .map_err(err)?
                    .into_iter()
                    .filter_map(|u| {
                        std::path::Path::new(&u.path)
                            .file_stem()
                            .map(|s| s.to_string_lossy().into_owned())
                    })
                    .collect();

            let families: Vec<commands::DuplicateFamily> = commands::duplicates(&g.store(scope))
                .map_err(err)?
                .into_iter()
                .filter(|f| f.vault == cfg.name)
                .collect();
            let at_risk: Vec<&String> = families
                .iter()
                .flat_map(|f| f.extras.iter())
                .filter(|id| outstanding.contains(*id))
                .collect();
            if !at_risk.is_empty() {
                return Err(format!(
                    "{} of these copies are not in git history yet, and deleting one would be \
                     unrecoverable. Record them first — then pruning is undoable.",
                    at_risk.len()
                ));
            }

            let mut removed = 0usize;
            let mut kept = 0usize;
            for f in &families {
                kept += 1;
                for id in &f.extras {
                    let parsed: fm_model::Id = match id.parse() {
                        Ok(i) => i,
                        Err(_) => continue,
                    };
                    commands::delete(&mut g.store(scope), &parsed.to_string()).map_err(err)?;
                    removed += 1;
                }
            }
            json(serde_json::json!({ "removed": removed, "kept": kept }))
        }
        "unrecorded" => {
            let g = lock()?;
            let configs = g.configs();
            let mut out: Vec<crate::dto::Unrecorded> = Vec::new();
            for cfg in &configs {
                let notes_rel = notes_rel_of(&cfg.path);
                let found = vcs::unrecorded(&cfg.path, &notes_rel).unwrap_or_default();
                if found.is_empty() {
                    continue;
                }
                let count_of =
                    |k: git::UnrecordedKind| found.iter().filter(|u| u.kind == k).count();
                // **Bounded.** A vault can hold thousands; the panel needs enough rows to recognise
                // what happened, not every filename. The counts above are the complete picture.
                // **Body counts first, over the whole set.** A duplicate count is only meaningful if
                // it counted everything, so this reads every outstanding note — small files, and the
                // alternative is a number that lies by omission. Capped so a pathological vault cannot
                // turn one command into ten thousand reads.
                let mut bodies: std::collections::HashMap<String, usize> =
                    std::collections::HashMap::new();
                for u in found.iter().take(crate::dto::UNRECORDED_SCAN) {
                    if let Some(b) = body_key(&cfg.path.join(&u.path)) {
                        *bodies.entry(b).or_insert(0) += 1;
                    }
                }
                let notes = found
                    .iter()
                    .take(crate::dto::UNRECORDED_DETAIL)
                    .map(|u| {
                        let full = cfg.path.join(&u.path);
                        let meta = std::fs::metadata(&full).ok();
                        crate::dto::UnrecordedNote {
                            id: std::path::Path::new(&u.path)
                                .file_stem()
                                .map(|s| s.to_string_lossy().into_owned())
                                .unwrap_or_default(),
                            path: u.path.clone(),
                            kind: match u.kind {
                                git::UnrecordedKind::New => "new",
                                git::UnrecordedKind::Modified => "modified",
                                git::UnrecordedKind::Deleted => "deleted",
                            }
                            .to_string(),
                            // The note's own title if it parses, else its first non-empty body line —
                            // the same "name it by something a human recognises" rule the conflict
                            // labels use. A deleted note has no file to read, and says so by `None`.
                            title: title_of(&full),
                            bytes: meta.as_ref().map(|m| m.len()),
                            modified: meta
                                .as_ref()
                                .and_then(|m| m.modified().ok())
                                .map(crate::dto::stamp_of),
                            role: role_of(&full),
                            // **The note's own `created`, beside the file's mtime.** They answer
                            // different questions and I conflated them: mtime says when the *file* was
                            // written, which a copy, a restore or a migration resets wholesale. Every
                            // one of 147 rows showing the same minute looked like a one-minute burst of
                            // writes; it is equally consistent with the whole set being copied into
                            // this vault in one minute, weeks after the notes were made. `created`
                            // tells the two apart, and without it the timestamp column invites exactly
                            // the wrong conclusion.
                            created: created_of(&full),
                            copies: body_key(&full)
                                .and_then(|b| bodies.get(&b).copied())
                                .unwrap_or(1),
                        }
                    })
                    .collect();
                out.push(crate::dto::Unrecorded {
                    vault: cfg.name.clone(),
                    count: found.len(),
                    new: count_of(git::UnrecordedKind::New),
                    modified: count_of(git::UnrecordedKind::Modified),
                    deleted: count_of(git::UnrecordedKind::Deleted),
                    notes,
                });
            }
            json(out)
        }
        // Record them. Explicit, never on a timer: this stages files the app does not remember
        // writing, and that is a decision a human should make rather than a debounce.
        "record_unrecorded" => {
            let g = lock()?;
            let cfg = g.config(scope, &s("vault"))?;
            let notes_rel = notes_rel_of(&cfg.path);
            // Same ordering as `commit`, for the same reason: `unrecorded` answers "nothing" for a
            // directory that is not a repository, so without this the one button that rescues
            // unstaged notes reported *"Nothing left to record"* to a user whose vault had never
            // been a repo — every note outstanding, and the surface saying there was nothing to do.
            vcs::ensure_repo(&cfg.path).map_err(err)?;
            let found = vcs::unrecorded(&cfg.path, &notes_rel).map_err(err)?;
            if found.is_empty() {
                return json(serde_json::json!({ "committed": false, "notes": 0 }));
            }
            let paths: Vec<std::path::PathBuf> =
                found.iter().map(|u| cfg.path.join(&u.path)).collect();
            let message = format!(
                "notes: recording {} note(s) the app had not staged",
                found.len()
            );
            let made = vcs::commit_all(&cfg.path, &message, &paths).map_err(err)?;
            // **When git declines, say why.** `commit_all` answers `bool`, and every reason it can
            // return `false` for collapses into that one value: an unmerged path, an unchanged tree,
            // nothing staged. The caller then reported `false` as *"Nothing left to record"* — so with
            // 147 notes outstanding and one note mid-merge, the button reassured the owner that there
            // was nothing to do, forever. A silent refusal on the one action that rescues unrecorded
            // notes is worse than an error, because it looks like success.
            //
            // The reasons are recomputed here rather than threaded through `commit_all`'s signature:
            // this is the only caller that needs to explain itself (a debounced auto-commit returning
            // `false` is the normal, quiet case), and the checks are the same two conditions the
            // backend tested, read back after the fact.
            let reason = if made {
                String::new()
            } else {
                let unmerged = vcs::conflicted(&cfg.path).map_err(err)?;
                if !unmerged.is_empty() {
                    format!(
                        "{} note(s) in this vault are mid-merge. Git refuses to commit anything \
                         until those are resolved — open Conflicts and settle them first.",
                        unmerged.len()
                    )
                } else if vcs::identity(&cfg.path).is_none() {
                    "This vault has no git identity, so nothing can be committed. Set your name \
                     and email in Backup settings."
                        .to_string()
                } else {
                    format!(
                        "Git accepted no change for the {} note(s) found. Nothing was lost — the \
                         notes are still on this device.",
                        found.len()
                    )
                }
            };
            json(serde_json::json!({
                "committed": made,
                "notes": found.len(),
                "reason": reason,
            }))
        }
        // The collaboration read-model: who last edited each note, and when, straight from each
        // vault's git log — one command behind the authorship labels, the activity stream, and
        // the contributor filter. Aggregated across vaults, newest-first.
        "activity" => {
            let since = {
                let s = s("since");
                if s.is_empty() {
                    "1 year ago".to_string()
                } else {
                    s
                }
            };
            // **The revwalk runs with the guard dropped.** This arm is among the first things the
            // UI's first `refresh()` fires, and it used to hold the vault lock across one `git log`
            // per vault — so on a cold start every other command queued behind a year of history.
            // The git half needs no store and the resolution needs no git, so they are separated:
            // `vcs::activity` here, `commands::resolve_touches` after the guard is re-taken.
            //
            // Fail-fast is preserved deliberately: one vault's git erroring still fails the whole
            // command, as it did before.
            let vaults = lock()?.configs();
            let mut touched = Vec::new();
            for cfg in &vaults {
                touched.push((cfg.clone(), vcs::activity(&cfg.path, &since).map_err(err)?));
            }

            let mut g = lock()?;
            let mut all = Vec::new();
            for (_cfg, touches) in touched {
                all.extend(commands::resolve_touches(&g.store(scope), touches).map_err(err)?);
            }
            drop(g);
            all.sort_by(|a, b| b.time.cmp(&a.time));
            json(all)
        }
        // A paper from a pasted BibTeX entry, DOI, arXiv id, URL or bare title — all offline.
        // The core links no HTTP client (`fm-agent-run`'s ruling), so nothing here resolves an
        // identifier; it recognises one, and reads a citation the user already holds.
        "create_paper" => {
            let mut g = lock()?;
            let into = g.config(scope, &s("vault"))?;
            json(
                commands::create_paper(&mut g.store(scope), &s("input"), &into.name)
                    .map_err(err)?,
            )
        }
        "paper_bibtex" => {
            json(commands::paper_bibtex(&lock()?.store(scope), &s("id")).map_err(err)?)
        }
        "capture" => {
            // Validate the target vault up front (unknown name → a loud error, never a
            // silent default), then route the note into it — the create-side twin of
            // `ingest`. Empty picks the default vault.
            let mut g = lock()?;
            let into = g.config(scope, &s("vault"))?;
            json(commands::capture(&mut g.store(scope), &s("body"), &into.name).map_err(err)?)
        }
        // Notes nothing has touched lately, read from git. Per vault, because history is per
        // repo — and a vault with no history is *skipped*, not reported as entirely stale:
        // "no evidence" and "old" are different answers and only one of them is true.
        "stale" => {
            let since = {
                let s = s("since");
                if s.is_empty() {
                    "90 days ago".to_string()
                } else {
                    s
                }
            };
            let mut g = lock()?;
            let mut all = Vec::new();
            for cfg in g.configs() {
                match commands::stale(&g.store(scope), &cfg.path, &since) {
                    Ok(rows) => all.extend(rows),
                    // A vault without git is not a failure of the whole query — the others
                    // still have an honest answer.
                    Err(fm_core::StoreError::Io(_)) => continue,
                    Err(e) => return Err(err(e)),
                }
            }
            json(all)
        }
        // Discussion, as notes. No `vault` argument on either, deliberately: a reply joins the
        // vault of the note it is about, because a vault is an audience.
        "reply" => {
            let mut g = lock()?;
            let meta = commands::reply(&mut g.store(scope), &s("id"), &s("body")).map_err(err)?;
            // A collaborator identity (the agent passes its model's) commits *this one message* under
            // it right now, so its authorship is the agent, not the vault's default — the same
            // git-author provenance a human collaborator gets from their own clone (Ruling 14).
            // Humans omit it and ride the normal batched commit under their own identity. Committing
            // only the message file leaves the user's other pending writes untouched, and a re-commit
            // by the later batch is a no-op (nothing changed). Best-effort: never fails a reply.
            let (an, ae) = (s("authorName"), s("authorEmail"));
            if !an.is_empty() && !ae.is_empty() {
                if let Ok(cfg) = g.config(scope, &meta.vault) {
                    let mine: Vec<std::path::PathBuf> = g
                        .all
                        .written(&cfg.name)
                        .into_iter()
                        .filter(|p| p.to_string_lossy().contains(&meta.id))
                        .collect();
                    if !mine.is_empty() {
                        let _ = vcs::commit_all_as(
                            &cfg.path,
                            &format!("message from {an}"),
                            &mine,
                            &an,
                            &ae,
                        );
                    }
                }
            }
            json(meta)
        }
        "thread" => json(commands::thread(&lock()?.store(scope), &s("id")).map_err(err)?),
        // A first-class discussion — a note that is the root of its own thread. `vault` is the
        // audience it joins (validated up front, unknown name refused), like `capture`.
        "create_discussion" => {
            let mut g = lock()?;
            let into = g.config(scope, &s("vault"))?;
            json(
                commands::create_discussion(&mut g.store(scope), &s("title"), &into.name)
                    .map_err(err)?,
            )
        }
        // Propose a change to an existing note: the change lands on a `proposal/<id>` branch (never
        // `main`) and a `proposes:` note records it for the Collaboration view. The target's *own*
        // vault decides the path and the size guardrails, so resolve it from the note first — and,
        // like `ingest`/`copy_note`, resolve the config to an owned value so `&mut g.store(scope)` and the
        // config can coexist. Refused (never truncated) when over a limit.
        "create_proposal" => {
            let id = s("id");
            let mut g = lock()?;
            let note_id = id.parse().map_err(|_| format!("invalid id: {id}"))?;
            let vault_name = g
                .store(scope)
                .get(note_id)
                .map_err(err)?
                .ok_or_else(|| format!("no such note: {id}"))?
                .vault;
            let cfg = g.config(scope, &vault_name)?;
            let limits = fm_core::descriptor::Descriptor::read(&cfg.path)
                .map_err(err)?
                .proposal_limits;
            // An agent passes its model's identity (`authorName`/`authorEmail`) so the proposal is
            // attributed to the model; a person proposing by hand sends neither and falls back to the
            // vault's own identity. A git author is a *label*, never a trust boundary (the human merge
            // gate is the sole authority), so accepting it from the caller is fine.
            let author_name = args.get("authorName").and_then(Value::as_str);
            let author_email = args.get("authorEmail").and_then(Value::as_str);
            let author = author_name.zip(author_email);
            // The reviewer's own sentence about what they changed, optional by design: a required
            // prompt produces satisficing, not reasons, and a skipped one is itself data. It becomes
            // the commit message body — never the subject, whose prefix is the squash barrier.
            // The supervision record. Every field is optional and every one is impossible to
            // reconstruct later: a person proposing by hand supplies none of them, an agent supplies
            // the input half that its proposal is otherwise an answer without.
            let sources: Vec<String> = args
                .get("sources")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let rec = commands::Record {
                why: args.get("why").and_then(Value::as_str),
                tool: args.get("tool").and_then(Value::as_str),
                query: args.get("query").and_then(Value::as_str),
                sources: &sources,
                kind: args.get("kind").and_then(Value::as_str),
            };
            let made = commands::create_proposal(
                &mut g.store(scope),
                &cfg.path,
                &id,
                &s("body"),
                &limits,
                author,
                rec,
            )
            .map_err(err)?;
            // Commit the proposal *note* (best-effort) so the Collaboration feed lists it after a
            // restart; the branch is already its own commit.
            if vcs::available() {
                let paths = g.all.written(&vault_name);
                if vcs::commit_all(&cfg.path, "backup: proposal", &paths).unwrap_or(false) {
                    g.all.clear_written(&vault_name);
                }
            }
            json(made)
        }
        // The read half of review: a proposal's branch diff against `main`. Resolve the proposal
        // note's own vault (immutable store read, so no `&mut` juggling), then diff.
        "proposal_diff" => {
            let id = s("id");
            let mut g = lock()?;
            let pid = id.parse().map_err(|_| format!("invalid id: {id}"))?;
            let vault_name = g
                .store(scope)
                .get(pid)
                .map_err(err)?
                .ok_or_else(|| format!("no such proposal: {id}"))?
                .vault;
            let cfg = g.config(scope, &vault_name)?;
            json(commands::proposal_diff(&g.store(scope), &cfg.path, &id).map_err(err)?)
        }
        "proposal_content" => {
            // The proposed note (host + title + body on the branch) — for the review to show and edit.
            let id = s("id");
            let mut g = lock()?;
            match id
                .parse()
                .ok()
                .and_then(|pid| g.store(scope).get(pid).ok().flatten())
            {
                Some(obj) => {
                    let path = g.config(scope, &obj.vault)?.path.clone();
                    json(commands::proposal_content(&g.store(scope), &path, &id).map_err(err)?)
                }
                None => json(serde_json::Value::Null),
            }
        }
        // Accept a proposal from the review surface — the GUI's merge button, since a UI-only user has
        // no `git merge`. Resolve the vault as `proposal_diff` does, merge the branch into main, then
        // re-read (the merge wrote the accepted note behind the index's back, like a pull). Fail-closed:
        // a conflicted merge is reported, not forced, and main is left untouched.
        "accept_proposal" => {
            let id = s("id");
            let mut g = lock()?;
            let pid = id.parse().map_err(|_| format!("invalid id: {id}"))?;
            let vault_name = g
                .store(scope)
                .get(pid)
                .map_err(err)?
                .ok_or_else(|| format!("no such proposal: {id}"))?
                .vault;
            let path = g.config(scope, &vault_name)?.path.clone();
            let outcome = commands::accept_proposal(&g.store(scope), &path, &id).map_err(err)?;
            g.store(scope).reindex(Reindex::Incremental).map_err(err)?;
            json(serde_json::json!({
                "outcome": match outcome {
                    fm_core::git::Accepted::Merged => "merged",
                    fm_core::git::Accepted::Conflicted => "conflicted",
                    fm_core::git::Accepted::AlreadyGone => "already_gone",
                }
            }))
        }
        "proposal_for" => {
            // The note's current open proposal (its PR), or null — so the note's own view can show it.
            let id = s("id");
            let mut g = lock()?;
            match id
                .parse()
                .ok()
                .and_then(|nid| g.store(scope).get(nid).ok().flatten())
            {
                Some(obj) => {
                    let path = g.config(scope, &obj.vault)?.path.clone();
                    json(commands::proposal_for(&g.store(scope), &path, &id).map_err(err)?)
                }
                None => json(serde_json::Value::Null),
            }
        }
        "proposal_shown" => {
            // Fire-and-forget from the review surface: it records that a human actually looked, which
            // is what separates "left alone" from "never displayed". Idempotent, so a re-render costs
            // one read and no write, and it must never fail the screen it is reporting about.
            let id = s("id");
            let mut g = lock()?;
            let wrote = commands::mark_proposal_shown(&mut g.store(scope), &id).map_err(err)?;
            if wrote {
                g.store(scope).reindex(Reindex::Incremental).map_err(err)?;
                // Only on the first sighting, so this is one commit per proposal and not one per
                // render — and, like the reject record, it must not depend on the tab outliving it.
                let vault_name = g
                    .store(scope)
                    .get(id.parse().map_err(|_| format!("invalid id: {id}"))?)
                    .map_err(err)?
                    .map(|o| o.vault)
                    .unwrap_or_default();
                if vcs::available() {
                    if let Ok(cfg) = g.config(scope, &vault_name) {
                        let path = cfg.path.clone();
                        let paths = g.all.written(&vault_name);
                        if vcs::commit_all(&path, "backup: proposal", &paths).unwrap_or(false) {
                            g.all.clear_written(&vault_name);
                        }
                    }
                }
            }
            nothing()
        }
        "reject_proposal" => {
            let id = s("id");
            let mut g = lock()?;
            let pid = id.parse().map_err(|_| format!("invalid id: {id}"))?;
            let vault_name = g
                .store(scope)
                .get(pid)
                .map_err(err)?
                .ok_or_else(|| format!("no such proposal: {id}"))?
                .vault;
            let path = g.config(scope, &vault_name)?.path.clone();
            let why = args.get("why").and_then(Value::as_str);
            commands::reject_proposal(&mut g.store(scope), &path, &id, why).map_err(err)?;
            g.store(scope).reindex(Reindex::Incremental).map_err(err)?;
            // Commit the record, exactly as `create_proposal` commits the note it writes. Left to
            // the debounced auto-commit this could be lost outright: that timer is a browser
            // `setTimeout` that dies with the tab, and with `FM_AUTO_SHUTDOWN` closing the tab *is*
            // how the app is quit. A rejection's reason is the only record of *why* — the rejected
            // text never reaches the branch — so it must not depend on the user staying around.
            if vcs::available() {
                let paths = g.all.written(&vault_name);
                if vcs::commit_all(&path, "backup: proposal", &paths).unwrap_or(false) {
                    g.all.clear_written(&vault_name);
                }
            }
            nothing()
        }
        // Every first-class discussion, newest-active first, enriched with who has posted in it.
        // Roots + counts come from the store (so they list even with no git); participants are
        // read from each vault's git log by hand — a discussion root and its replies are all
        // messages, and `activity` deliberately drops messages, so their authorship is gathered
        // here rather than reused.
        "discussions" => {
            let mut g = lock()?;
            let mut summaries = commands::discussions(&g.store(scope)).map_err(err)?;
            // Fill participants from each vault's git log, merging across vaults. A wide `--since`
            // because the roots are already listed by the store — this only adds authorship, and
            // an old-but-still-open discussion deserves its participants. No git → no authors, the
            // discussion still lists with its title and vault.
            // `@0` (git's epoch-seconds date) means "since the beginning" — NOT `1970-01-01`, which
            // git parses as local-midnight and underflows to 1969 UTC in positive-offset timezones,
            // where `git log --since` then wrongly returns *nothing* (git 2.53). `@0` is timezone-safe.
            for cfg in g.configs() {
                let Ok(mut who) =
                    commands::discussion_participants(&g.store(scope), &cfg.path, "@0")
                else {
                    continue;
                };
                for sum in &mut summaries {
                    if let Some(list) = who.remove(&sum.root.id) {
                        sum.participants = list;
                    }
                }
            }
            json(summaries)
        }
        // Every thread with messages — first-class discussions AND ordinary notes with comment
        // threads — for the study agent to watch. Not the Discussions view (that is `discussions`).
        "thread_roots" => json(commands::thread_roots(&lock()?.store(scope)).map_err(err)?),
        "set_property" => {
            commands::set_property(&mut lock()?.store(scope), &s("id"), &s("key"), &s("value"))
                .map_err(err)?;
            nothing()
        }
        // Answers with the new **version** — the content hash of the body just written, which
        // the caller holds and sends back as `base` on its next write. That round trip is the
        // lost-update guard for an editor that has been open across someone else's pull. See
        // `commands::update_body`, which compares `base` against `version_of(&obj.body)`.
        //
        // It is a hash, **not** the `updated` stamp this comment used to name. The stamp was the
        // original design (changed 2026-07-18) and the stale wording here and in `ipc.ts` is what
        // taught two UI conflict handlers to send one back — which no hash can ever equal, so
        // those notes conflicted permanently and silently stopped being saved.
        "update_body" => {
            let stamp =
                commands::update_body(&mut lock()?.store(scope), &s("id"), &s("body"), &s("base"))
                    .map_err(err)?;
            json(stamp)
        }
        "delete" => {
            commands::delete(&mut lock()?.store(scope), &s("id")).map_err(err)?;
            nothing()
        }
        // Searched across vaults: the reference names bytes, not a place. Configs are
        // cloned out and the guard dropped before touching the disk.
        "asset_status" => {
            let r = s("reference");
            let (vaults, default) = {
                let g = lock()?;
                (g.configs(), g.config(scope, "").ok())
            };
            let found = vaults.iter().find_map(|v| {
                commands::asset_status(&v.path, &r)
                    .ok()
                    .filter(|st| st.has_blob)
            });
            match found {
                Some(st) => json(st),
                // Absent everywhere. Still not an error — "media absence is a warning,
                // never an error" — so answer with the default vault's honest "no". With
                // no vaults at all there is nothing to be honest *about*, and no note can
                // exist to reference it, so the default "no" is the whole answer.
                None => match default {
                    Some(v) => json(commands::asset_status(&v.path, &r).map_err(err)?),
                    None => json(commands::AssetStatus {
                        has_blob: false,
                        has_thumb: false,
                        mime: None,
                    }),
                },
            }
        }
        "resolve_asset" => {
            let (r, k) = (s("reference"), s("kind"));
            let vaults = lock()?.configs();
            vaults
                .iter()
                .find_map(|v| commands::resolve_asset_bytes(&v.path, &r, &k).ok())
                .map(Output::Bytes)
                .ok_or_else(|| format!("blob not present in any vault: {r}"))
        }
        "open_external" => {
            // Resolve, then drop the guard: handing a path to the OS can block on anything.
            // Never the thumbnail: "open this" means the file, not our small copy of it.
            let path = lock()?.find_blob(scope, &s("reference"), false)?;
            host.open_external(&path)?;
            nothing()
        }
        "commit" => {
            // Hold the lock so a commit can't snapshot the vault mid-write (a server is
            // thread-per-connection). One guard, where this used to take two: resolving
            // the vault through the same guard is what keeps that from being a
            // self-deadlock now that the store and the list share a lock.
            let mut g = lock()?;
            let cfg = g.config(scope, &s("vault"))?;
            // Exactly the files this app wrote or deleted — not a directory, and certainly
            // not `-A`. A vault may also be a repo you commit to yourself, and this fires
            // five seconds after every save.
            let mut paths = g.all.written(&cfg.name);
            // **Plus anything outstanding this process has no memory of writing.**
            //
            // The write list is per-process, and that precision is deliberate. But the owner's
            // report is the case it fails: *"When I press backup it should commit and push. I do
            // not commit as a user, that is a background concept."* Backup is the only durability
            // affordance in the product, so a commit that records only what *this* run happened to
            // write is not sufficient — and on Android, where the app is normally left by being
            // killed, "this run" is usually a few minutes of work while the backlog is everything
            // before it. That backlog reached 180 notes on the owner's phone, in a vault with a
            // remote, and pressing Backup did not clear it.
            //
            // `App::load` already adopts them at open, which fixes the *next* commit after a
            // relaunch. It does not help a session that has been running since before the notes
            // appeared, and it made "is my work recorded?" depend on when the process happened to
            // start — which is exactly the background concept a user should never have to hold.
            // Doing it here makes the answer independent of that.
            //
            // **The safety argument is unchanged and is the naming scheme**, not the memory: a
            // file at `<notes dir>/<ULID>.md` is what `FileStore` writes and nothing else
            // produces, so this can never sweep up a hand-written file or someone's staged work.
            // It is the same filter `adoptable` has applied at open since 2026-07-31; only the
            // moment is new.
            //
            // Costs one `git status` over the notes dir per commit, on a path already debounced to
            // once per five seconds, and only for paths git does not already have.
            //
            // **The repository has to exist before we ask git what it is missing.** `unrecorded`
            // answers "nothing" for a directory that is not a repo — correctly, git cannot say
            // otherwise — and `commit_all` only creates the repo once it is already running. So on
            // the very first backup of a new vault the sweep below found nothing, `commit_all`
            // init'd the repo, staged the `.gitignore`/`.gitattributes` it had just written, and
            // answered **`committed: true`** with not one note in history. The user is told their
            // vault is backed up; `commitStep` carries a no-conflict result straight on to the push
            // and reports "synced"; and the notes only *become* visibly unrecorded afterwards,
            // because now there is a repo to be missing from. That is `outstanding.md` §2.9 — a
            // surface that will not say what it knows — and it is one line: ask first.
            //
            // Idempotent and free on every later commit (`ensure_repo` returns early on a `.git`
            // that exists); `commit_all` still calls it, so this is an ordering fix, not a second
            // responsibility. On a machine with no git at all it fails here instead of a few lines
            // down, with the same error.
            vcs::ensure_repo(&cfg.path).map_err(err)?;
            for extra in adoptable(&cfg.path) {
                if !paths.contains(&extra) {
                    paths.push(extra);
                }
            }
            let made = vcs::commit_all(&cfg.path, &s("message"), &paths).map_err(err)?;
            // Cleared only once the commit actually landed: a failed commit that forgot its
            // list would leave those notes unstaged forever.
            if made {
                g.all.clear_written(&cfg.name);
            }
            // **Why nothing was committed matters.** `commit_all` refuses outright during a
            // conflicted merge — correctly, since staging conflict markers would enshrine
            // them — but it says so with the same `false` it uses for "clean tree, nothing
            // to do". Those two are opposites: one is a no-op, the other is *every
            // subsequent write silently never being committed*, for as long as the conflict
            // sits there. Answering with the conflicted notes lets the sync loop stop and
            // name them instead of reporting "synced" over a frozen vault.
            //
            // Gated on the vault actually being mid-merge, because `vcs::conflicts` shells out
            // to `git status --porcelain` — a second one, since `commit_all` already ran it —
            // and "committed nothing" is the *overwhelmingly* common answer for a debounced
            // auto-commit over a clean tree. `.git/MERGE_HEAD` is git's own marker for an
            // unfinished merge, so a stat answers the question for free in the common case.
            let conflicts = if made || !cfg.path.join(".git/MERGE_HEAD").exists() {
                Vec::new()
            } else {
                vcs::conflicts(&cfg.path).unwrap_or_default()
            };
            json(CommitResult {
                committed: made,
                conflicts,
            })
        }
        // Answers **what the snapshot contained**, not merely that one happened. See `BackupRun`.
        "backup" => json(run_backup(app, scope, &s("vault"))?),
        // What the two backup tiers would actually do right now — the panel needs
        // this to promise the user only what it can deliver.
        "backup_status" => json(backup_status(app)?),
        // **"When did this last work?"** — deliberately separate from `backup_status`, which
        // polls. See `backup_latest`.
        "backup_latest" => json(backup_latest(app, scope, &s("vault"))?),
        // The identity rides along because this is the one moment it is worth
        // asking for: a vault gaining a remote is a vault gaining an audience, and
        // from here on every commit carries a name into somebody else's clone.
        // Empty means "don't touch it" — a user whose git is already configured is
        // never asked, so the panel sends nothing.
        "set_supervision" => {
            // Two answers, always written together and explicitly — an absent key and a deliberate
            // "no" must not look identical to whoever has to answer for it later.
            let path = lock()?.config(scope, &s("vault"))?.path;
            let flag = |k: &str| args.get(k).and_then(Value::as_bool);
            let current = fm_core::descriptor::Descriptor::read(&path)
                .map(|d| d.supervision)
                .unwrap_or_default();
            let sup = fm_core::descriptor::Supervision {
                collect: flag("collect").unwrap_or(current.collect),
                publish: flag("publish").unwrap_or(current.publish),
            };
            fm_core::descriptor::Descriptor::set_supervision(&path, sup).map_err(err)?;
            // Hand back the refreshed list, as `set_git_assets_max` does: the caller re-renders
            // from the truth on disk rather than from what it hoped it wrote.
            let g = lock()?;
            json(infos(&g.configs(), &g.all.names()))
        }
        "set_git_remote" => {
            let (name, email) = (s("name"), s("email"));
            // Resolved once, where it used to be resolved twice.
            let path = lock()?.config(scope, &s("vault"))?.path;
            if !name.is_empty() || !email.is_empty() {
                vcs::set_identity(&path, &name, &email).map_err(err)?;
            }
            vcs::set_remote(&path, &s("url")).map_err(err)?;
            nothing()
        }
        // **Identity on its own, with no remote in sight.**
        //
        // `set_git_remote` above can also set it, and until now that was the *only* way — fine
        // while the one moment worth asking was a vault gaining an audience. A first-run welcome
        // screen asks earlier, and for a different reason: git refuses to commit anything at all
        // without a committer, so someone who never adds a remote still needs this, or their
        // first backup fails citing a name nobody ever asked them for.
        //
        // A separate command rather than letting `set_git_remote` take an empty URL: that would
        // leave a command whose name promises a remote quietly not setting one, and the next
        // reader would have to run it to find out which it did.
        "set_identity" => {
            let path = lock()?.config(scope, &s("vault"))?.path;
            vcs::set_identity(&path, &s("name"), &s("email")).map_err(err)?;
            nothing()
        }
        "push" => {
            // Same reason as `commit`: don't let a push snapshot the vault mid-write.
            let g = lock()?;
            let path = g.config(scope, &s("vault"))?.path;
            json(vcs::push_squashed(&path, &s("message")).map_err(err)?)
        }
        // Bring a collaborator's work home. Holds the lock for the same reason push does
        // — a merge rewrites notes under the app's feet, and the very next incremental
        // reindex is what makes them visible.
        "pull" => {
            let mut g = lock()?;
            let path = g.config(scope, &s("vault"))?.path;
            let outcome = vcs::pull(&path).map_err(err)?;
            // The merge just wrote files behind the index's back. Re-read now rather
            // than leave the user staring at pre-pull content until the next heartbeat.
            g.store(scope).reindex(Reindex::Incremental).map_err(err)?;
            json(match outcome {
                git::Pulled::UpToDate => PullResult {
                    merged: 0,
                    conflicts: Vec::new(),
                },
                git::Pulled::Merged(n) => PullResult {
                    merged: n,
                    conflicts: Vec::new(),
                },
                git::Pulled::Conflicted(f) => PullResult {
                    merged: 0,
                    conflicts: f,
                },
            })
        }
        // The browser heartbeat, which doubles as **the local poll**. Liveness is the
        // transport's business and was already refreshed before dispatch; the answer here
        // is the other half: has the vault moved under us?
        //
        // `get`/`query` serve SQLite, and a full reindex only runs at `open` — so
        // without this a `git pull`, a merge driver, or an edit in Vim is *invisible*
        // to a running app. Liveness is a *separate* beat (`POST /api/alive`, lock-free);
        // this one is the poll, at 15 s and only while the tab is visible. They were split
        // precisely because riding one beat forced this to run every 3 s, taking the vault
        // lock each time. Quiet is the common case and quiet is a stat per file.
        //
        // **`since` is the client's last generation**, and the answer is a comparison rather
        // than a report. The drift flag below is still read — it is what notices Vim and the
        // merge driver — but it is *converted into* a generation bump, because a flag derived
        // from mtimes can only be true once: this very reindex writes the fresh mtimes back,
        // so a second client asking a moment later would be told nothing had happened. A
        // counter can be read by any number of clients, each at its own pace.
        //
        // A client with no `since` (0, a fresh tab) is told `changed: false` — it has just
        // loaded everything anyway — and takes the current generation to compare against next
        // time.
        "ping" => {
            let since = args.get("since").and_then(Value::as_u64).unwrap_or(0);
            let mut g = lock()?;
            let drift = g.store(scope).reindex(Reindex::Incremental).map_err(err)?;
            drop(g);
            if drift.updated > 0 || drift.removed > 0 {
                app.bump();
            }
            let generation = app.generation.load(Ordering::Relaxed);
            let mut g = lock()?;
            json(Ping {
                changed: generation > since,
                generation,
                git: vcs::available(),
                restic: backup::available(),
                // Taken from the store rather than from `changed`, because this is the
                // *current* set across every vault, labelled by which one — not just what
                // this pass happened to re-read.
                skipped: g
                    .store(scope)
                    .skipped()
                    .iter()
                    .map(SkippedOut::from)
                    .collect(),
                unopened_vaults: g
                    .all
                    .unopened()
                    .iter()
                    .map(|(name, why)| format!("{name}: {why}"))
                    .collect(),
            })
        }
        // Hand an unreadable note to whatever the platform thinks owns `.md`. This is the
        // one thing you can actually *do* about a conflicted merge from inside the app:
        // the note does not parse, so no editor of ours can open it.
        //
        // The vault+name pair is looked up in the *current* skipped set, and the path comes
        // from there — never from the caller. So the only files this can open are ones the
        // indexer just reported as broken, and a stale name from a panel left open since
        // before the fix fails closed rather than opening something else.
        "open_skipped" => {
            // Resolve, then drop the guard: handing a path to the OS can block on anything.
            let (vault, name) = (s("vault"), s("name"));
            // Resolve, then drop the guard: handing a path to the OS can block on anything.
            let path = skipped_path(&mut lock()?, scope, &vault, &name)?;
            host.open_external(&path)?;
            nothing()
        }
        // The **in-app** half of the skipped-note surface — the raw editor decisions.md #4 names as the
        // fix for a note that will not render (a genuine frontmatter conflict, or corrupt YAML). Unlike
        // `open_skipped`, which hands the path to an external editor, these work on **any device** —
        // the phone has no external editor — by reading and rewriting the raw file text directly.
        //
        // Same security model as `open_skipped`: the path is resolved from the store's *current
        // skipped set*, never from the caller, so neither can touch a file that is not a
        // currently-unreadable note. `resolve_skipped` writes exactly what it is given (files-as-truth,
        // byte-for-byte) and does not commit — the normal save/commit flow finishes the merge once the
        // note is readable again — but it reports whether the new text parses, so the UI can tell
        // "resolved" from "still has conflict markers".
        "read_skipped" => {
            let (vault, name) = (s("vault"), s("name"));
            let path = skipped_path(&mut lock()?, scope, &vault, &name)?;
            let text = std::fs::read_to_string(&path).map_err(err)?;
            json(serde_json::json!({ "text": text }))
        }
        "resolve_skipped" => {
            let (vault, name) = (s("vault"), s("name"));
            let path = skipped_path(&mut lock()?, scope, &vault, &name)?;
            std::fs::write(&path, s("text")).map_err(err)?;
            let parses = fm_core::frontmatter::from_file(&s("text")).is_ok();
            json(serde_json::json!({ "parses": parses }))
        }
        // The audiences that exist. `[]` is **the first-run signal** — the one command
        // that is meaningful with no vaults, and the reason it isn't folded into
        // `backup_status` (which shells out per vault, including a network `ls-remote`).
        // **Scoped, and it is the one place the scope is visible to the user.** The vault
        // switcher, the copy-to menu and the create-target picker are all built from this, so a
        // list that included audiences the caller cannot read would offer destinations every
        // write then refuses — and would leak the names themselves, which are not nothing: a
        // vault called "acquisition-2027" discloses whether it holds anything or not.
        "list_vaults" => {
            let g = lock()?;
            let mine: Vec<VaultConfig> = g
                .configs()
                .into_iter()
                .filter(|c| scope.allows(&c.name))
                .collect();
            let names: Vec<&str> = g
                .all
                .names()
                .into_iter()
                .filter(|n| scope.allows(n))
                .collect();
            json(infos(&mine, &names))
        }
        // **Which attachments travel with this vault's notes.** Written into the vault's own
        // `vault.json`, so the rule follows the vault to every device and every collaborator
        // rather than living in one browser's settings.
        //
        // `max` is a size a person writes ("2MB"), or empty/absent to turn it off. Parsed here
        // rather than in the UI so the CLI and any future frontend get the same grammar.
        "set_git_assets_max" => {
            let vault = s("vault");
            let raw = s("max");
            let max = match raw.trim() {
                "" | "off" | "none" => None,
                t => Some(fm_core::descriptor::parse_size(t).ok_or_else(|| {
                    format!("{t:?} is not a size — try 2MB, 500kB, or leave it empty for none")
                })?),
            };
            // The guard is dropped before touching the disk, and retaken to report — the same
            // shape every other arm here uses, so a slow filesystem never blocks a `ping`.
            let path = lock()?.config(scope, &vault)?.path;
            fm_core::descriptor::Descriptor::set_git_assets_max(&path, max)
                .map_err(|e| e.to_string())?;
            let g = lock()?;
            json(infos(&g.configs(), &g.all.names()))
        }
        // What would happen if we created a vault here — the form asks on every keystroke.
        "check_path" => {
            let path = resolve_path(&s("name"), &s("path"))?;
            let g = lock()?;
            json(check_path(
                &g,
                app.config.as_deref(),
                app.config_writable,
                &s("name"),
                &path,
            ))
        }
        "create_vault" => json(create_vault(
            app,
            &s("name"),
            &resolve_path(&s("name"), &s("path"))?,
        )?),
        // What importing this folder would involve — asked on every keystroke, like `check_path`.
        // The guard is taken only for the destination question; the directory walk runs with it
        // released, because a source path is whatever a person typed and may be enormous.
        "check_import" => {
            let (vault, name) = (s("vault"), s("name"));
            let path = resolve_path(&name, &s("path"))?;
            let problem = {
                let g = lock()?;
                destination_problem(&g, app, scope, &vault, &name, &path)
            };
            json(import_check(&s("source"), problem))
        }
        // **A fourth way a vault gets filled**, beside create/clone/restore — and the only one
        // that *converts* rather than moving bytes. See `run_import` for where the lock is.
        "run_import" => json(run_import(
            app,
            scope,
            &s("source"),
            &s("vault"),
            &s("name"),
            &resolve_path(&s("name"), &s("path"))?,
            args.get("stubs").and_then(Value::as_bool).unwrap_or(false),
        )?),
        // **Stop showing me this vault.** The fourth verb the vault list needed: three commands
        // brought a vault into being (`create_vault`, `clone_vault`, `restore_vault`) and none took
        // one away, so a vault the app had auto-created — the phone makes an empty default one on
        // first launch — could never be got rid of from inside the product. For a user who only ever
        // sees the UI that is not a papercut, it is permanent (owner, 2026-07-31: "creates only
        // confusion").
        "forget_vault" => json(forget_vault(app, &s("name"))?),
        // What this installation is actually configured as — the answer to "what am I
        // operating with?". Read-only by construction and by necessity: `vaults::save` is
        // append-only and never rewrites an existing entry, so a settings screen that offered
        // to edit a vault's path or restic repo would silently no-op. Where something *is*
        // editable, it stays where it already is (the backup panel owns remotes and identity).
        //
        // Deliberately cheap: vault list, config file, environment and capabilities, no
        // shelling out. `backup_status` answers remotes and identities and is the slowest
        // command in the app — a settings screen must not be a reason to run it.
        "config" => {
            let g = lock()?;
            json(Config {
                version: option_env!("FM_VERSION").unwrap_or("dev").to_string(),
                vault_list: app.config.as_ref().map(|p| p.display().to_string()),
                vault_list_writable: app.config_writable,
                vaults: infos(&g.configs(), &g.all.names()),
                restic: g
                    .configs()
                    .iter()
                    .map(|c| VaultRestic {
                        vault: c.name.clone(),
                        repo: c.restic.clone(),
                    })
                    .collect(),
                env: [
                    "FM_VAULT",
                    "FM_VAULTS",
                    "FM_CONFIG_DIR",
                    "FM_RESTIC_REPO",
                    "FM_ADDR",
                    "FM_UI_DIST",
                    "FM_AUTO_SHUTDOWN",
                ]
                .iter()
                .filter_map(|k| {
                    std::env::var(k).ok().map(|v| EnvVar {
                        name: (*k).into(),
                        value: v,
                    })
                })
                .collect(),
                // Capabilities, not settings: things the machine either has or does not, which
                // change what the app can do and are the commonest source of "why is this
                // greyed out". `RESTIC_PASSWORD` is reported as present/absent only — never
                // its value, which is why it is a bool and not an `env` entry.
                git: fm_core::vcs::available(),
                restic_installed: backup::available(),
                // Reported for the same reason git and restic are: a feature that quietly does not
                // work is one you discover on the day you needed it.
                pdf_text: fm_core::ingest::pdf_text_available(),
                // Env **or** the password formicaria keeps: setting one in the app has to move
                // this, or the panel goes on saying "no password" about a machine that has one.
                restic_password_set: crate::secrets::has_restic_password(),
                vault_root: vaults::vault_root().map(|p| p.display().to_string()),
                ca_bundle: crate::ca_bundle::status(),
                platform: std::env::consts::OS,
            })
        }
        // The other way a vault comes into existence: someone else already has it. Same
        // registration as `create_vault`, with a clone in front and an identity behind.
        "clone_vault" => json(clone_vault(
            app,
            &s("name"),
            &resolve_path(&s("name"), &s("path"))?,
            &s("url"),
            &s("gitName"),
            &s("gitEmail"),
        )?),
        // Can we reach this repo, and if not, why not — asked *before* a clone commits to a
        // directory. The three failures a clone cannot tell apart (typo, no credentials,
        // offline) need three different next steps.
        "probe_remote" => json(probe_remote(&s("url"))),
        // Give this machine a credential for a private repo. **Where it goes depends on the
        // platform, and that is the point** — a desktop hands it to git's own helper and keeps
        // nothing; a phone has no helper, so the app keeps it.
        "set_git_credential" => {
            let (url, token) = (s("url"), s("token"));
            if token.trim().is_empty() {
                return Err("paste a token — an empty one would just fail at push time".into());
            }
            if fm_core::git::available() {
                // The username is nearly always ignored for a PAT; every forge accepts any
                // non-empty value beside it. Default to the conventional one.
                let user = {
                    let u = s("username");
                    if u.trim().is_empty() {
                        "x-access-token".to_string()
                    } else {
                        u
                    }
                };
                fm_core::git::credential_approve(&url, &user, &token).map_err(err)?;
            } else {
                crate::secrets::save_token(&token)?;
            }
            json(git_auth(&url))
        }
        // **Media backup, configurable from the app at last.** Both halves of it were env vars and
        // hand-edited JSON: Settings said *"there is no UI for it"* about the repo, and the
        // documented answer for the password was *"a launcher you have edited yourself"*. A tier of
        // backup only reachable by editing a startup script is not a feature this product has.
        "set_restic_repo" => {
            let (vault, repo) = (s("vault"), s("repo"));
            let config = app.config.clone().ok_or(
                "there is nowhere to save the vault list on this machine — set FM_VAULTS"
                    .to_string(),
            )?;
            let mut g = lock()?;
            // Resolved through the scope, so a paired device cannot reconfigure an audience it
            // was never given — and a name that is not ours is refused before anything is written.
            let cfg = g.config(scope, &vault)?;
            // Materialise first: an `FM_VAULT`-only install has no file yet, and `set_restic`
            // edits an entry rather than inventing one. `save` leaves every known entry alone,
            // so this cannot disturb a list that already exists.
            let list = g.configs();
            vaults::save(&list, &config)?;
            let repo = Some(repo.trim())
                .filter(|r| !r.is_empty())
                .map(str::to_string);
            // The file first: if it fails nothing has changed, which is the recoverable order.
            vaults::set_restic(&cfg.name, repo.as_deref(), &config)?;
            g.set_restic(&cfg.name, repo);
            drop(g);
            json(backup_status(app)?)
        }
        // One password for every repo — a per-vault one would multiply the places a secret lives,
        // and it is deliberately not written into `vaults.json`, which is a file of paths a user
        // may reasonably open or send someone while debugging.
        "set_restic_password" => {
            crate::secrets::save_restic_password(&s("password"))?;
            json(backup_status(app)?)
        }
        "clear_restic_password" => {
            crate::secrets::clear_restic_password()?;
            json(backup_status(app)?)
        }
        "clear_git_credential" => {
            crate::secrets::clear_token()?;
            json(git_auth(&s("url")))
        }
        // Where credentials come from here, and whether there already are any for this URL.
        "git_auth" => json(git_auth(&s("url"))),
        // The third way in: a vault you already have, in a backup, on a machine that no
        // longer exists. Same registration as the other two, with a restic restore in front.
        "restore_vault" => json(restore_vault(
            app,
            &s("name"),
            &resolve_path(&s("name"), &s("path"))?,
            &s("repo"),
        )?),
        // The user's saved `.view` files, aggregated across every vault: a view is
        // git-tracked *in* the vault it belongs to, but the query it defines runs against
        // the whole set (so `prop: vault` can narrow, or a dashboard can span audiences).
        // A view that won't parse is listed with its error, never dropped.
        // **Saving what you arranged.** Until now a `.view` could be neither written nor deleted
        // from the UI, so the only documented way to have one was to author YAML in a text editor —
        // for a headline feature, in an app whose owner works only through the UI. This saves the
        // arrangement (renderer, and a board's grouping); editing a filter stays a file-level job
        // and `views::save_view` refuses to silently drop one.
        // **The write is only half the job.** `commit` stages the store's write list, not a
        // directory scan, so a `.view` written straight to disk is never recorded — and the save
        // dialog has always promised the opposite ("so it travels with your notes"). Enrolling the
        // path here is what makes that sentence true.
        //
        // The write list's usual safety argument is the ULID naming scheme; this widens it. The
        // justification is narrower, not broader: these are paths **this process just wrote, this
        // second**, which is the list's original meaning. It can never sweep up a hand-written file.
        "save_view" => {
            // One guard for the whole arm. Resolving the vault and then seeding the write list
            // through two separate locks would let a commit slip between them and miss the file.
            let mut g = lock()?;
            let cfg = g.config(scope, &s("vault"))?;
            let (name, path) = (cfg.name.clone(), cfg.path.clone());
            let renderer: crate::views::Renderer = serde_json::from_value(
                args.get("view")
                    .cloned()
                    .unwrap_or(Value::String("board".into())),
            )
            .map_err(|_| "that is not a view kind this app can render".to_string())?;
            let group = s("group_by");
            let tag = s("tag");
            let written = crate::views::save_view(
                &path,
                &s("name"),
                renderer,
                if group.is_empty() {
                    None
                } else {
                    Some(group.as_str())
                },
                if tag.is_empty() {
                    None
                } else {
                    Some(tag.as_str())
                },
            )?;
            g.all.seed_written(&name, vec![written]);
            json(views_of(&name, &path))
        }
        // Deleting has to be recorded for the same reason, and a little more urgently: an
        // unrecorded deletion comes back on the next pull.
        "delete_view" => {
            let mut g = lock()?;
            let cfg = g.config(scope, &s("vault"))?;
            let (name, path) = (cfg.name.clone(), cfg.path.clone());
            let removed = crate::views::delete_view(&path, &s("name"))?;
            g.all.seed_written(&name, vec![removed]);
            json(views_of(&name, &path))
        }
        // ---- Themes. A sibling of the view commands above, for the same reasons: a file in the
        // vault, named by a label a person typed, recorded in git so it reaches the other machine.
        // `MASTERPLAN.md` has specified `themes/*.css` since the start; this is that, built.
        "list_themes" => {
            let g = lock()?;
            let mut all = Vec::new();
            for v in g.configs() {
                // Stamp the vault on the way past — `list_themes` is handed a path and has no name
                // to report, and without it "delete" cannot find a theme outside the default vault.
                all.extend(themes_of(&v.name, &v.path));
            }
            json(all)
        }
        "read_theme" => {
            let path = lock()?.config(scope, &s("vault"))?.path;
            json(crate::themes::read_theme(&path, &s("name"))?)
        }
        "save_theme" => {
            let mut g = lock()?;
            let cfg = g.config(scope, &s("vault"))?;
            let (name, path) = (cfg.name.clone(), cfg.path.clone());
            let written = crate::themes::save_theme(&path, &s("name"), &s("css"))?;
            g.all.seed_written(&name, vec![written]);
            json(themes_of(&name, &path))
        }
        "delete_theme" => {
            let mut g = lock()?;
            let cfg = g.config(scope, &s("vault"))?;
            let (name, path) = (cfg.name.clone(), cfg.path.clone());
            let removed = crate::themes::delete_theme(&path, &s("name"))?;
            g.all.seed_written(&name, vec![removed]);
            json(themes_of(&name, &path))
        }
        // Renaming is its own command, never save-under-the-new-name plus delete: that
        // composition slips past `save_view`'s refuse-don't-flatten guard (a new name hits no
        // existing file) and would destroy a hand-written filter. Both paths reach the write list,
        // or git sees half a rename and the old file returns on the next pull.
        "rename_view" => {
            let mut g = lock()?;
            let cfg = g.config(scope, &s("vault"))?;
            let (name, path) = (cfg.name.clone(), cfg.path.clone());
            let (from, to) = crate::views::rename_view(&path, &s("from"), &s("to"))?;
            g.all.seed_written(&name, vec![from, to]);
            json(views_of(&name, &path))
        }
        "rename_theme" => {
            let mut g = lock()?;
            let cfg = g.config(scope, &s("vault"))?;
            let (name, path) = (cfg.name.clone(), cfg.path.clone());
            let (from, to) = crate::themes::rename_theme(&path, &s("from"), &s("to"))?;
            g.all.seed_written(&name, vec![from, to]);
            json(themes_of(&name, &path))
        }
        "list_views" => {
            let g = lock()?;
            let mut all = Vec::new();
            for v in g.configs() {
                // Stamp the vault on the way past. `list_views` takes a path and has no name to
                // report; this loop is the only place both are in hand.
                all.extend(views_of(&v.name, &v.path));
            }
            json(all)
        }
        // Run one view by name. The file is read from whichever vault holds it; the query
        // runs against the full store. A parse error surfaces as the error body, so a broken
        // view says why rather than silently returning nothing.
        "run_view" => {
            let name = s("name");
            let mut g = lock()?;
            let vault_path = g
                .configs()
                .iter()
                .find(|v| {
                    crate::views::list_views(&v.path)
                        .iter()
                        .any(|vi| vi.name == name)
                })
                .map(|v| v.path.clone())
                .ok_or_else(|| format!("no view named '{name}'"))?;
            json(crate::views::run_view(&g.store(scope), &vault_path, &name).map_err(err)?)
        }
        // **Chunked upload, one slice at a time.** The raw `body` is this chunk; `session` and
        // `seq` say which upload and which position. See `fm_core::chunked` for why the session
        // lives at `<vault>/.fm-ingest/` and why `seq` is checked rather than trusted.
        //
        // Bounds the transient at one chunk regardless of the file's size, which is what lifts
        // the 16 MB ceiling `ingest` below still needs — that one holds the whole file in memory
        // several times over, so its limit is a memory limit wearing a size limit's clothes.
        "ingest_chunk" => {
            let session = s("session");
            let seq = args.get("seq").and_then(Value::as_u64).unwrap_or(0) as u32;
            let g = lock()?;
            let into = g.config(scope, &s("vault"))?;
            let total = fm_core::chunked::append(&into.path, &session, seq, body).map_err(err)?;
            // The running total is the caller's progress bar and its correctness check: a
            // frontend that has sent more than the file weighs knows something went wrong
            // without waiting for the finish.
            json(serde_json::json!({ "session": session, "received": total }))
        }
        // Assemble, ingest, and write the asset note — the same note `ingest` writes, from the
        // same function, because the only real difference between the two paths is how the bytes
        // arrived.
        "ingest_finish" => {
            let name = {
                let n = s("name");
                if n.is_empty() { "asset".to_string() } else { n }
            };
            json(ingest_unlocked(
                app,
                scope,
                &s("vault"),
                &name,
                IngestSource::Session(&s("session")),
            )?)
        }
        // Abandon an upload and reclaim its bytes — what a cancel button calls. Not required for
        // correctness (the sweep would get it) and offered anyway, because "cancel" that leaves
        // a gigabyte on a phone until tomorrow is not a cancel.
        "ingest_cancel" => {
            let g = lock()?;
            let into = g.config(scope, &s("vault"))?;
            fm_core::chunked::discard(&into.path, &s("session")).map_err(err)?;
            nothing()
        }
        // Binary upload: the raw `body` IS the file, which is exactly why its name and
        // vault arrive as `args` rather than in it.
        "ingest" => {
            let name = {
                let n = s("name");
                if n.is_empty() {
                    "asset".to_string()
                } else {
                    n
                }
            };
            // Into the vault the caller names — the blob lands beside the notes that
            // will reference it, and never in an audience that shouldn't have it. Both
            // the path and the name, so the bytes and the asset note land in the *same*
            // vault: splitting them puts the file in one audience and its note in another.
            //
            // The owned `config` is what makes this borrow-check: `&mut g.store(scope)` and a
            // `&VaultConfig` borrowed from the same guard cannot coexist.
            // **Zero bytes is a transport failure, not a file.** Storing it silently is what made
            // a phone photo unrenderable: the empty blob hashes to `e3b0c442…b855`, ingest
            // reported success, and the note got a reference to nothing. Every capture produced
            // the same hash and nothing said so.
            //
            // Refused here rather than only in the UI because this is the seam every frontend
            // crosses, and the cost is asymmetric: attaching a genuinely empty file gains
            // nothing, while accepting one hides a broken byte path behind a success message.
            if body.is_empty() {
                return Err(format!(
                    "{name} arrived with no bytes — nothing was stored. The file did not reach \
                     the vault; this is a transport problem, not a problem with the file."
                ));
            }
            json(ingest_unlocked(
                app,
                scope,
                &s("vault"),
                &name,
                IngestSource::Bytes(body),
            )?)
        }
        // Copy a note into another vault. Restrictive by default (only the prose travels);
        // `with_assets` opts in to carrying the first-degree blobs. Validate the target up
        // front, hand `copy_note` every vault's (name, path) so it can locate/copy blobs, then
        // version the new note in its own vault (best-effort — a solo user's own repo).
        "copy_note" => {
            let with_assets = args
                .get("with_assets")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let mut g = lock()?;
            let into = g.config(scope, &s("vault"))?;
            let vault_paths: Vec<(String, PathBuf)> =
                g.configs().into_iter().map(|c| (c.name, c.path)).collect();
            let result = commands::copy_note(
                &mut g.store(scope),
                &s("id"),
                &into.name,
                &vault_paths,
                with_assets,
            )
            .map_err(err)?;
            if vcs::available() {
                let paths = g.all.written(&into.name);
                if vcs::commit_all(&into.path, "backup: copy note", &paths).unwrap_or(false) {
                    g.all.clear_written(&into.name);
                }
            }
            json(result)
        }
        // Pre-check for the copy popover: does the target vault already hold a copy of this
        // note? Drives the "this will replace the existing copy" warning.
        "copy_status" => {
            let mut g = lock()?;
            let into = g.config(scope, &s("vault"))?;
            json(commands::copy_status(&g.store(scope), &s("id"), &into.name).map_err(err)?)
        }
        // Recede a copy: delete the copied note from the target vault and take back the blobs
        // this copy newly wrote (only those nothing else there still references).
        "uncopy_note" => {
            let blobs: Vec<String> = args
                .get("blobs")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let mut g = lock()?;
            let into = g.config(scope, &s("vault"))?;
            let vault_paths: Vec<(String, PathBuf)> =
                g.configs().into_iter().map(|c| (c.name, c.path)).collect();
            commands::uncopy_note(
                &mut g.store(scope),
                &s("id"),
                &into.name,
                &blobs,
                &vault_paths,
            )
            .map_err(err)?;
            if vcs::available() {
                let paths = g.all.written(&into.name);
                if vcs::commit_all(&into.path, "backup: undo copy", &paths).unwrap_or(false) {
                    g.all.clear_written(&into.name);
                }
            }
            nothing()
        }
        other => Err(format!("unknown command: {other}")),
    }
}

/// Where a blob really is — resolved across every vault, for a transport that wants to
/// stream the bytes itself rather than take them through [`Output::Bytes`].
///
/// This is the read path behind a streaming blob route: `resolve_asset` buffers the whole
/// file to hand it back, which is the wrong shape for a 300 MB video. Handing back the
/// *path* lets the shell open it and stream.
pub fn blob_path(app: &App, scope: &Scope, reference: &str) -> Result<PathBuf, String> {
    app.lock()?.find_blob(scope, reference, false)
}

/// The blob route's resolver, able to ask for the derived thumbnail. Scoping is unchanged — the
/// vault is still chosen by which one holds the blob, so a new query parameter cannot reach an
/// audience the caller was not given.
pub fn blob_path_of_kind(
    app: &App,
    scope: &Scope,
    reference: &str,
    thumb: bool,
) -> Result<PathBuf, String> {
    app.lock()?.find_blob(scope, reference, thumb)
}

impl Vaults {
    /// **The vaults this caller is an audience for**, as a `Store`.
    ///
    /// Every read path in `dispatch` goes through here, which is the point: narrowing the set a
    /// query *runs over* is the only version that cannot be forgotten. Filtering results after a
    /// federated read would mean the other audience's notes had already been loaded to be
    /// dropped again, and every future read path would have to remember to do it.
    ///
    /// [`Scope::All`] — every caller that existed before a tablet could pair, including
    /// `fm-cli`, the phone and the study agent — costs nothing: `Scoped` with no list delegates
    /// straight through.
    fn store<'a>(&'a mut self, scope: &'a Scope) -> Scoped<'a> {
        Scoped::new(&mut self.all, scope.names())
    }

    /// A named vault; an empty name means the default (the first).
    ///
    /// Returns an **owned** config, not a reference: a `&VaultConfig` borrowed from the
    /// guard would conflict with `&mut self.all` in the very arms that need both
    /// (`ingest`), and it is three allocations against a caller that is usually about to
    /// fork `git`.
    ///
    /// An **unknown** name is an error, never a fallback. Quietly writing a note meant
    /// for "lab" into "personal" is a disclosure that git history makes permanent, and
    /// the reverse silently loses the note — so a typo has to be loud. With **no** vaults
    /// there is no default to fall back to either, which is the first run and says so.
    ///
    /// **This is where "which vault" is decided, so it is where the scope has to bite.** The
    /// arms resolve the target here and hand the *name* onward, which means a scoped caller
    /// would never reach `Scoped`'s own routing check — `capture` with no vault would resolve
    /// to `list[0]` and file a tablet's note into an audience it cannot read. A vault outside
    /// the scope is reported exactly as one that does not exist: a caller must not be able to
    /// probe for the names of audiences it was not given.
    fn config(&self, scope: &Scope, name: &str) -> Result<VaultConfig, String> {
        let mut mine = self.list.iter().filter(|v| scope.allows(&v.name));
        if self.list.is_empty() {
            return Err("no vaults configured — create one first".into());
        }
        if name.is_empty() {
            // The first vault **this caller can see**, which for `Scope::All` is `list[0]` and
            // therefore unchanged.
            return mine
                .next()
                .cloned()
                .ok_or_else(|| "no vault is shared with this device".to_string());
        }
        mine.find(|v| v.name == name)
            .cloned()
            .ok_or_else(|| format!("no vault named '{name}'"))
    }

    /// An owned snapshot, so callers can drop the guard before doing I/O — which
    /// `backup_status` must, since it shells out to `git ls-remote` per vault and would
    /// otherwise block every `ping` for a network round trip.
    fn configs(&self) -> Vec<VaultConfig> {
        self.list.clone()
    }

    /// Bring a vault into the live set — the whole point being that creating one must not
    /// need a restart. Pushes **both** halves, so `store` and `list` cannot disagree, and
    /// **appends**: `list[0]` is the default that receives every fresh capture, so
    /// inserting would silently move where new notes land.
    fn add(&mut self, cfg: VaultConfig, store: fm_core::FileStore) {
        self.all.add(store);
        self.list.push(cfg);
    }

    /// Point a vault's media backup at a repository, in the live set.
    ///
    /// The **memory** half of `vaults::set_restic`; the file is the other half. Both, or the
    /// running app disagrees with the file it will reload from — the same failure `forget`
    /// orders its two steps to avoid.
    fn set_restic(&mut self, name: &str, repo: Option<String>) -> bool {
        match self.list.iter_mut().find(|c| c.name == name) {
            Some(c) => {
                c.restic = repo;
                true
            }
            None => false,
        }
    }

    /// Drop a vault from the live set. Both halves again, for the same reason.
    fn remove(&mut self, name: &str) -> bool {
        let had = self.all.remove(name);
        let before = self.list.len();
        self.list.retain(|c| c.name != name);
        had || self.list.len() != before
    }

    /// Where a blob really is. **Every vault is searched**, because a `sha256:`
    /// reference deliberately does not say which vault holds the bytes — and it should
    /// not: that is what keeps a cross-vault `note:`/`asset:` link free, and what lets
    /// ULIDs be the only identifier anyone needs. Content-addressing makes searching
    /// *correct* rather than merely convenient: whichever vault answers, the bytes hash
    /// to the reference, so they are the same bytes. Vault-scoping references would
    /// re-couple a note to a location and break the links.
    ///
    /// **That reasoning holds only for a caller entitled to every vault**, which was every
    /// caller until a device could pair. Content-addressing cuts the other way here: blobs are
    /// deduplicated by hash, so a reference a collaborator legitimately learns from the shared
    /// vault resolves against the *private* vault's blob store too — the same bytes, and
    /// therefore the same answer, from an audience they were never given. So the search is over
    /// the vaults this caller can see. `Scope::All` restores the original behaviour exactly.
    fn find_blob(&self, scope: &Scope, reference: &str, thumb: bool) -> Result<PathBuf, String> {
        for v in self.list.iter().filter(|v| scope.allows(&v.name)) {
            if let Ok(p) = commands::blob_path_of_kind(&v.path, reference, thumb) {
                return Ok(p);
            }
        }
        Err(format!("blob not present in any vault: {reference}"))
    }
}

/// Resolve a currently-unreadable note's path from the store's skipped allowlist — the shared security
/// gate for `open_skipped`/`read_skipped`/`resolve_skipped`. The path comes from the indexer's own
/// skipped set, never from the caller, so none of the three can be tricked into naming an arbitrary
/// file (a traversal, `/etc/passwd`, a readable note in another vault). Takes the live lock guard so
/// all three arms resolve against the same borrowed set.
///
/// **Scoped, and that is a fourth thing it gates.** The skipped set spans every vault, so without
/// this a paired device could name an unreadable note in an audience it was never given and be
/// handed its path — the same disclosure the traversal guard exists to prevent, arriving by the
/// front door.
fn skipped_path(
    g: &mut MutexGuard<'_, Vaults>,
    scope: &Scope,
    vault: &str,
    name: &str,
) -> Result<PathBuf, String> {
    g.store(scope)
        .skipped()
        .iter()
        .find(|sk| sk.vault == vault && sk.name == name)
        .map(|sk| sk.path.clone())
        .ok_or_else(|| {
            format!(
                "not a currently-unreadable note: {vault}/{name} — it may have been fixed already"
            )
        })
}

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

/// The views in one vault, each stamped with the vault that holds it.
///
/// **Every arm that returns a list must go through this.** The `list_*` arms stamp while iterating
/// the configs; a write arm has only one vault in hand and it is easy to return the bare list — but
/// the UI keeps whatever comes back, so an unstamped reply silently blanks the vault on every row
/// and the *next* delete or rename resolves against the default vault instead. Found by running the
/// built binary and reading the JSON, not by a test.
fn views_of(name: &str, path: &std::path::Path) -> Vec<crate::views::ViewInfo> {
    crate::views::list_views(path)
        .into_iter()
        .map(|mut v| {
            v.vault = name.to_string();
            v
        })
        .collect()
}

/// The themes in one vault, stamped. Same rule, same reason as [`views_of`].
fn themes_of(name: &str, path: &std::path::Path) -> Vec<crate::themes::ThemeInfo> {
    crate::themes::list_themes(path)
        .into_iter()
        .map(|mut t| {
            t.vault = name.to_string();
            t
        })
        .collect()
}

fn json<T: serde::Serialize>(v: T) -> Result<Output, String> {
    serde_json::to_vec(&v)
        .map(Output::Json)
        .map_err(|e| e.to_string())
}

/// Success with nothing to say. An empty JSON payload, not `null` — the commands that use
/// this report by not failing.
fn nothing() -> Result<Output, String> {
    Ok(Output::Json(Vec::new()))
}

/// The heartbeat's answer: did anything change that the tab is not showing?
/// A bool, not a count — the UI's only choice is whether to re-run its query.
#[derive(serde::Serialize)]
struct Ping {
    changed: bool,
    /// The vaults' current generation — see [`App::generation`]. The client stores it and
    /// sends it back as `since` on the next beat; `changed` above is just `generation > since`.
    ///
    /// It is returned rather than kept server-side because the server has no idea how many
    /// clients there are, and must not start guessing: a per-client cursor would need identity,
    /// eviction, and a definition of "gone". The client holding its own cursor makes N clients
    /// cost exactly nothing, and a client that misses a beat catches up on the next one.
    generation: u64,
    /// Whether this machine has git at all. **Not a dependency — a capability.** The tab
    /// uses it to stop firing an auto-commit every 5s at a binary that isn't there, and
    /// to say so once instead of failing silently forever. Rides the heartbeat because
    /// the check is cached and the tab already beats.
    git: bool,
    /// Whether restic is on this machine — the same kind of claim as `git`, for the same
    /// reason, and now load-bearing rather than cosmetic: restoring a vault from a backup is
    /// one of the ways a vault is acquired, and the form must not offer a route it cannot
    /// take. Android is precisely this case — it has no restic and never will — so the option
    /// is absent there rather than present and failing.
    restic: bool,
    /// Notes that could not be read, as `vault: filename: why`.
    ///
    /// **The app knew this and only told a terminal.** A vault opens even when a note is
    /// unparseable — usually a conflicted merge, and refusing to start would take away the
    /// app you need to fix it — but those notes are then absent from *every* view. That was
    /// printed to stderr at startup, which in a browser-first product means nobody sees it:
    /// the note is simply gone, with no reason given and no way to ask.
    ///
    /// Rides the heartbeat rather than being its own command because it must stay current —
    /// a conflicted note appears mid-session, when a pull lands, not at startup. Almost
    /// always empty, so it costs a `[]` per beat.
    skipped: Vec<SkippedOut>,
    /// **Vaults that are configured and could not be opened**, as `name: why`.
    ///
    /// Exactly the same argument as `skipped` one field up, one level out: `MultiStore::open` used to
    /// refuse *every* vault when one would not open — the whole app, over one folder — and the escape
    /// hatch it documents ("delete `index.sqlite` and reopen") needs a shell the phone does not have.
    /// It now opens the rest, which is only an improvement if the missing one is *named*: a vault
    /// that silently is not there is indistinguishable from data loss to the person who put notes in
    /// it. Rides the heartbeat for the same reason — a vault can become unopenable mid-session (an
    /// unplugged drive, a directory moved) — and is almost always empty.
    unopened_vaults: Vec<String>,
}

/// One unreadable note, as the UI sees it.
///
/// **Deliberately without the path.** The panel does not need it — it names the note back
/// to `open_skipped`, which resolves the path itself — and a filesystem path is not
/// something to hand a browser for free. Keeping the resolution server-side is also what
/// makes the skipped set an allowlist rather than an argument: there is no path a caller
/// can name, so there is no traversal to guard against.
#[derive(serde::Serialize)]
struct SkippedOut {
    vault: String,
    name: String,
    reason: String,
}

impl From<&fm_core::SkippedNote> for SkippedOut {
    fn from(s: &fm_core::SkippedNote) -> Self {
        Self {
            vault: s.vault.clone(),
            name: s.name.clone(),
            reason: s.reason.clone(),
        }
    }
}

/// What a commit did. `committed: false` with a non-empty `conflicts` is the case worth
/// distinguishing: the vault is mid-merge, so **nothing will be committed until a human
/// settles it** — not "there was nothing to commit".
#[derive(serde::Serialize)]
struct CommitResult {
    committed: bool,
    conflicts: Vec<String>,
}

/// What a pull did. `conflicts` non-empty is a *result*, not an error: those notes have
/// markers in their body (the `.md` driver keeps them out of the frontmatter), so they
/// still open in the editor for a human to settle.
#[derive(serde::Serialize)]
struct PullResult {
    merged: u32,
    conflicts: Vec<String>,
}

/// A vault, as the sidebar and the first-run screen need it. Deliberately **not**
/// [`VaultStatus`]: that one is about a *remote* and shells out to `git ls-remote` per
/// vault, and making the first-run screen — the thing shown when no vaults exist — depend
/// on the app's slowest, git-flavoured command would be backwards.
/// What the machine is configured as. See the `config` arm for why this is read-only.
#[derive(serde::Serialize)]
struct Config {
    /// Which build this is — the release tag, or `dev` for anything built locally.
    ///
    /// **Baked in at build time, because the crates have no version of their own.** The
    /// workspace is `0.0.0` and always has been; the real version exists only as the git tag the
    /// release workflow builds from, so it arrives through `FM_VERSION` and falls back to `dev`
    /// when nobody set it. `option_env!`, not `env!`: an ordinary `cargo build` must not fail for
    /// want of a release variable.
    ///
    /// **Why it is here at all.** Each release unpacks into its own folder and the vault lives
    /// inside it, so a user updating ends up with two formicaria folders and no way to tell which
    /// one they are looking at — from inside the app there was no version anywhere. A string to
    /// read, nothing more: no comparison, no update check, nothing that phones home.
    version: String,
    /// The vault list file we would write, or `None` when this machine has no config
    /// directory at all — in which case nothing can be persisted, which is worth saying.
    vault_list: Option<String>,
    /// False also means "we could not parse what is there", not merely "no permission" — and
    /// in that case we will never overwrite it. Both are worth showing.
    vault_list_writable: bool,
    vaults: Vec<VaultInfo>,
    restic: Vec<VaultRestic>,
    env: Vec<EnvVar>,
    git: bool,
    /// Restic on this machine, distinct from `restic_password_set` (configured) and from
    /// `restic` above (which vaults name a repo). Three different questions that used to be
    /// answerable only as one, which is how a panel enables a control for a tool that is
    /// not installed.
    restic_installed: bool,
    /// Whether the text inside a PDF can be read out and made searchable.
    pdf_text: bool,
    restic_password_set: bool,
    /// The directory this installation puts vaults in, or `null` when the user chooses their
    /// own. **Present on a phone, absent on a desktop** — and it is what tells the form
    /// whether to ask for a folder at all. See `vaults::vault_root`.
    vault_root: Option<String>,
    /// What happened when this build tried to give its statically-linked OpenSSL a CA trust
    /// store: a count, or the reason there is none. `null` on a desktop, which uses the
    /// system's store and never builds one.
    ///
    /// **Reported because the alternative was unreadable.** Android does not route Rust's
    /// stderr to logcat, so a startup diagnostic printed there is invisible — which is exactly
    /// how "the SSL certificate is invalid" stayed unexplained through two builds. Whether a
    /// trust store exists is the first question to ask when a remote will not verify, and the
    /// app is the only thing in a position to answer it.
    ca_bundle: Option<String>,
    /// Which OS this build is running on — `std::env::consts::OS`, so `"linux"`, `"macos"`,
    /// `"windows"`, `"android"` or `"ios"`.
    ///
    /// **A fact, deliberately, and not the policy that reads it.** The surface that needs this is
    /// the sideload notice: an iOS build of formicaria is signed with the user's own Apple ID and
    /// stops opening about seven days later (`decisions.md#track-m`, *iOS ships as an unsigned
    /// IPA*). A field called `sideloaded` would bake that route into the wire contract and be
    /// wrong the day the route changes; the OS name will still be true. The UI decides what a
    /// platform means.
    platform: &'static str,
}

/// A vault's restic destination. Its own type rather than a field on `VaultInfo` because it
/// comes from the config entry rather than the store — and because it is the one piece of
/// backup configuration with no UI to edit it anywhere, which is precisely why it is shown.
#[derive(serde::Serialize)]
struct VaultRestic {
    vault: String,
    repo: Option<String>,
}

/// An `FM_*` override actually in effect. Only these are reported: they change where data
/// lives or how the server binds, which is exactly what a confused user needs to see. No
/// secret appears here — the restic password is reported as a bool and never by value.
#[derive(serde::Serialize)]
struct EnvVar {
    name: String,
    value: String,
}

#[derive(serde::Serialize)]
struct VaultInfo {
    name: String,
    /// For display, so two vaults both called `notes` are tellable apart.
    path: String,
    /// Index 0 — where every fresh capture lands. The UI has to be able to say so.
    default: bool,
    /// Attachments up to this many bytes travel with this vault's notes. `None` — the default —
    /// means none do, which is the two-tier split the backup design rests on.
    ///
    /// Read from the vault's own `vault.json` rather than from app settings, because it decides
    /// what enters **shared, permanent history**: a per-device value would let the loosest
    /// machine choose for every collaborator, and a pushed commit cannot be un-pushed.
    git_assets_max: Option<u64>,
    /// **What to show instead of `name`** — the repository this vault is a clone of, when it has a
    /// remote: `…/formicarium-vault.git` → `formicarium-vault`. `None` for a vault with no remote,
    /// which keeps its local name.
    ///
    /// **Display only; `name` remains the identity.** A vault's name is not a label — it is the write
    /// routing key (`MultiStore::route`), the argument seventeen dispatch arms take, and the key
    /// behind the UI's persisted view preferences (`hiddenVaults`, `fm-board-order`,
    /// `fm-card-order`). Renaming a vault to match its remote would silently reset every one of those
    /// and break any command already in flight. So identity stays local and stable; only the label
    /// follows the remote. Same split as `authorKey`/label for contributors (`decisions.md#ui`).
    ///
    /// Why bother: the same repo cloned on two devices can carry two different local names — the
    /// owner's laptop and phone did — so one *audience* looked like two different vaults depending on
    /// which screen you were on. The remote is the thing both devices agree about.
    label: Option<String>,
    /// The committer this vault signs with, or `None` when git has never been told who you are.
    ///
    /// **Here rather than in `backup_status`, and that placement is the whole point.** The welcome
    /// screen gates on "does this person have an identity yet", which has to be answered on the
    /// path that renders the app — and `backup_status` runs a network `git ls-remote` per vault,
    /// the slowest command in the product. A first-run screen that waits on it is a first-run
    /// screen that waits on the network. This list already spawns git locally for `label`, so the
    /// answer costs one more local `git config` read beside a call that was happening anyway.
    identity: Option<fm_core::git::Identity>,
    /// **What may be done with this vault's review record** — collected locally, and published.
    ///
    /// Rides here for the same reason `identity` does: this list already reads the vault's
    /// descriptor for `git_assets_max`, so the answer costs nothing extra, and the alternative is a
    /// second command whose only job is to read one file.
    supervision: Supervision,
}

/// The wire shape of [`fm_core::descriptor::Supervision`] — two independent answers, never one
/// combined flag. Collapsing them is precisely the mistake that froze the only comparable open
/// corpus: its contributors had agreed to collection, and that was read as agreement to publish.
#[derive(serde::Serialize)]
struct Supervision {
    collect: bool,
    publish: bool,
}

/// What the create-vault form needs: the filesystem facts, plus the ones only the vault
/// list can answer, plus **the verdict**.
///
/// `ok` is computed here and not in Svelte on purpose. Duplicating the policy in the
/// browser is how you get a button that enables and then fails — which is exactly what
/// `restic_ready` did when it meant "configured" rather than "will work".
#[derive(serde::Serialize)]
struct PathCheck {
    #[serde(flatten)]
    facts: commands::PathFacts,
    name_ok: bool,
    name_taken: bool,
    path_taken: bool,
    /// The vault this path nests inside, or that nests inside it. Two `FileStore`s over
    /// one tree means one note in two audiences, indexed twice — the boundary the whole
    /// `MultiStore` design exists to make un-crossable.
    overlaps: Option<String>,
    /// Whether the vault list can be written at all. False → creating anything would be a
    /// vault that vanishes on restart, so the form must not offer it.
    config_writable: bool,
    ok: bool,
}

/// One vault's git standing. **Per vault, not per app** — one vault is one repo, one
/// remote, one collaborator list, so every field here is singular *about that vault* and
/// there is no honest way to collapse them. A single "unpushed" number across a set of
/// vaults would be a number about nothing.
#[derive(serde::Serialize)]
struct VaultStatus {
    /// The audience. Doubles as the argument every git command takes back.
    name: String,
    /// Where this vault's notes push to (`origin`), or null when unset.
    remote: Option<String>,
    /// Commits made here but not on the remote; null when never pushed.
    unpushed: Option<u32>,
    /// Who this vault's commits are signed by, or null when nobody real is — the panel
    /// asks for a name only when this is null, so anyone whose git is already configured
    /// never sees the question. Per vault on purpose: a vault is an audience, and the
    /// name on a lab repo need not be the one on your personal notes.
    identity: Option<git::Identity>,
    /// Someone else has pushed work we don't have. Null when unknowable (no remote,
    /// never pushed, or offline — a sleeping laptop is not an error). One `ls-remote`,
    /// which moves no refs: knowing must not itself be the thing that puts the vault
    /// in the state the ancestry guard has to survive.
    remote_moved: Option<bool>,
    /// Notes with conflict markers sitting in them, waiting for a human.
    conflicts: Vec<String>,
    /// Where this vault's media backs up to — a path or URL, so the UI can say whether
    /// it would leave this machine. **Never the password.** Null when this vault has no
    /// restic repo, which is not an error: a restic repo is per repository, so a set of
    /// vaults needs one each, and you may well not want one for all of them.
    restic_repo: Option<String>,
    /// This vault's media could actually be backed up **right now**: restic is installed,
    /// this vault has a repo, and a password is set. All three, because "ready"
    /// must mean "will work" — gating on configuration alone offers a checkbox that ticks
    /// and then fails on a machine with no restic.
    restic_ready: bool,
    /// The largest attachment this vault sends with its notes, or null for the default —
    /// notes only. **Here because the git tier cannot otherwise state its own scope.** The
    /// panel's promise line said "media is not included" unconditionally, which is false for
    /// any vault with a limit set: `commit_all` `git add -f`s every blob at or under it. The
    /// value is set in Settings and read here; a surface that says what a tier carries has to
    /// be told what the tier carries.
    git_assets_max: Option<u64>,
}

/// What each backup tier can do right now, per vault. fm-core stays free of environment
/// and configuration concerns, so the env-derived half is assembled here.
///
/// **Both tiers are per vault.** Git was always: one vault, one repo, one remote. And
/// restic is too, for the same shape of reason — a restic repo *is* per repository, so
/// backing up a set of vaults means a repo each. There is no app-wide media destination
/// to report, which is why there is no field here for one.
#[derive(serde::Serialize)]
struct BackupStatus {
    /// Every vault, in configured order; the first is the default. A single-vault
    /// install is a list of one, so the panel needs no separate shape for it.
    vaults: Vec<VaultStatus>,
    /// Whether this machine has git. Without it every vault below reports `remote: null,
    /// identity: null` — which is indistinguishable from "not set up yet", and would have
    /// the panel invite you to type a remote into a tier that cannot run. Per machine, not
    /// per vault: git is either installed or it isn't.
    git: bool,
    /// Whether this machine has restic. Per machine for the same reason as `git`. Split
    /// from `restic_ready` on purpose: "no restic installed" and "restic installed but this
    /// vault has no repo" are different sentences to say to someone.
    restic: bool,
    /// Whether this machine holds the restic password. **Per machine, like the tool itself** —
    /// one password unlocks every repository here, because a per-vault one would only multiply
    /// the places a secret lives.
    ///
    /// A bool and never the value: nothing downstream needs the password, so no shape here can
    /// leak it. It is the third of the three conditions `restic_ready` folds together, reported
    /// separately so the panel can name *which* one is missing instead of greying a row out.
    restic_password_set: bool,
}

/// When a vault's media was last snapshotted, and what that snapshot covered.
///
/// **Its own command, not a field on [`VaultStatus`].** `backup_status` is polled every 45 s and
/// already shells out per vault; asking restic for the latest snapshot costs another spawn each
/// time, and a `restic snapshots` against a network repo is a round trip. So the one fact a person
/// actually wants from a backup panel — *when did this last work* — is fetched when a human is
/// looking at it, not on a timer.
#[derive(serde::Serialize)]
struct LatestBackup {
    /// The vault this is about, echoed back so a caller can key several answers.
    vault: String,
    /// Restic's short snapshot id, or null when the repo has no `fm`-tagged snapshot yet.
    /// **Null is not an error**: a freshly configured repo has never been written to, and saying
    /// "never" is the useful answer rather than a failure.
    id: Option<String>,
    /// When it was taken, in restic's own words — an RFC 3339 stamp. Null with `id`.
    time: Option<String>,
    /// The **source** paths it recorded, absolute on whatever machine took it. Worth reporting
    /// because a snapshot taken on another device names that device's paths, and a restore that
    /// silently used them is the failure this makes visible before it happens.
    paths: Vec<String>,
    /// Why there is no answer, when there is none to be had — no restic, no repo, no password.
    /// Distinguished from `id: null` on purpose: *"never backed up"* and *"this machine cannot
    /// tell you"* are different sentences to put in front of someone.
    unavailable: Option<String>,
}

/// What a `backup` run put in the repository.
///
/// **The command used to answer nothing at all.** A snapshot taken thirty seconds ago told the
/// app only that it had happened, so the panel printed the same fixed phrase — *"notes and
/// attachments"* — over every vault, including the ones that have no attachments yet, and a
/// person watching a backup they cannot see had no way to tell a full snapshot from an empty
/// one. This is `outstanding.md` §2.10's last residue, and it is the level below *"last backed
/// up at"*: that says **when**, this says **what**.
#[derive(serde::Serialize)]
struct BackupRun {
    /// The vault this is about, echoed back as [`LatestBackup`] does — the panel runs this per
    /// vault and writes one line each.
    vault: String,
    /// The notes directory that went in, by name (`notes`, or whatever `vault.json` calls it).
    /// Null for a vault that has none yet.
    notes_dir: Option<String>,
    /// Whether `blobs/` existed and went in.
    blobs: bool,
    /// restic's account of the snapshot. **Null is "restic did not say", not "it was empty"** —
    /// the same distinction `backup_latest` draws between `id: null` and `unavailable`, and for
    /// the same reason: a zero a caller cannot tell from an unknown is worse than no number.
    contents: Option<SnapshotContents>,
}

/// Restic's own numbers for the snapshot it just wrote. Mirrors `fm_core::backup::Contents`.
#[derive(serde::Serialize)]
struct SnapshotContents {
    /// The short id, named the way `backup_latest` names it.
    id: String,
    files_new: u64,
    files_changed: u64,
    files_unmodified: u64,
    /// Bytes read out of the vault.
    bytes_processed: u64,
    /// Bytes the repository grew by — usually a fraction of the above, which is deduplication
    /// doing the work this tier exists for.
    bytes_added: u64,
}

/// The read behind *"last backed up at"*.
///
/// `fm_core::backup::latest()` has returned the newest `fm`-tagged snapshot since the tier was
/// built, and until now **no command exposed it** — so the fact could not be shown at any price.
fn backup_latest(app: &App, scope: &Scope, vault: &str) -> Result<LatestBackup, String> {
    let v = app.lock()?.config(scope, vault)?;
    let unavailable = |why: &str| LatestBackup {
        vault: v.name.clone(),
        id: None,
        time: None,
        paths: Vec::new(),
        unavailable: Some(why.to_string()),
    };
    if !backup::available() {
        return Ok(unavailable(
            "restic is not installed on this machine, so it cannot say when this vault was last \
             backed up",
        ));
    }
    let Some(repo) = v.restic.clone() else {
        return Ok(unavailable(
            "this vault has no media-backup repository configured, so there is nothing to ask",
        ));
    };
    let Some(password) = crate::secrets::restic_password() else {
        return Ok(unavailable(
            "no restic password is set on this machine, so the repository cannot be opened",
        ));
    };
    // A failure here is reported, not raised: an unreachable repo is an ordinary Tuesday for a
    // backup destination on another machine, and a panel that throws on it tells the user less
    // than one that says which repo it could not reach.
    match backup::latest(Path::new(&repo), &password) {
        Ok(Some(snap)) => Ok(LatestBackup {
            vault: v.name.clone(),
            id: Some(snap.id),
            time: Some(snap.time),
            paths: snap.paths.iter().map(|p| p.display().to_string()).collect(),
            unavailable: None,
        }),
        // The repo opened and holds no `fm`-tagged snapshot. **"Never" is an answer**, and it is
        // the one that should worry somebody, so it is not folded into `unavailable`.
        Ok(None) => Ok(LatestBackup {
            vault: v.name.clone(),
            id: None,
            time: None,
            paths: Vec::new(),
            unavailable: None,
        }),
        Err(e) => Ok(unavailable(&format!("could not read {repo}: {e}"))),
    }
}

fn backup_status(app: &App) -> Result<BackupStatus, String> {
    // One password for every repo. A per-vault password would have to live somewhere,
    // and the one place it must never live is the config file next to the paths.
    let has_password = crate::secrets::has_restic_password();
    // The tool itself. Media backup is an optional *feature*: no restic, no feature — but
    // the notebook is untouched, and the panel has to say which of those it is.
    let has_restic = backup::available();
    // Clone the configs out and **drop the guard** before any of this: every entry below
    // shells out per vault, including `remote_moved`'s network `ls-remote`. Holding the
    // lock across that would stall every `ping`, and with it the local poll, for a
    // round trip to GitHub. (Not the shutdown watchdog, despite what this comment used to
    // claim: a transport refreshes liveness *before* dispatch, and the watchdog reads only
    // that — so a slow command cannot make the app quit under you.)
    let vaults = app.lock()?.configs();
    let vaults = vaults
        .iter()
        .map(|v| VaultStatus {
            name: v.name.clone(),
            remote: vcs::remote(&v.path).unwrap_or(None),
            unpushed: vcs::unpushed(&v.path).unwrap_or(None),
            identity: vcs::identity(&v.path),
            remote_moved: vcs::remote_moved(&v.path).unwrap_or(None),
            conflicts: vcs::conflicts(&v.path).unwrap_or_default(),
            restic_ready: has_restic && v.restic.is_some() && has_password,
            restic_repo: v.restic.clone(),
            // Best-effort, exactly as the vault list treats it (`VaultInfo::git_assets_max`): a
            // descriptor that will not parse reports "off", the same as having no opinion. One
            // small JSON read per vault, beside the network `ls-remote` two lines up that
            // dominates this whole call.
            git_assets_max: fm_core::descriptor::Descriptor::read(&v.path)
                .ok()
                .and_then(|d| d.git_assets_max),
        })
        .collect();
    Ok(BackupStatus {
        vaults,
        git: vcs::available(),
        restic: has_restic,
        restic_password_set: has_password,
    })
}

/// Ingest bytes into a vault **without holding the vault lock across the subprocesses**.
///
/// `fm_core::ingest` shells out twice per file — `pdftotext` for the searchable text and
/// `vipsthumbnail` for the preview — and both used to run with the global guard held. A bulk
/// import therefore froze every tab, every pane and the phone's whole UI for as long as it took;
/// the estimate on the record is 25–40 minutes for 5,000 PDFs. `papers-plan.md` B5 named it, and
/// `ingest_finish` (added 2026-09-04 for chunked ingest) inherited it — which is the worse half,
/// since that path exists precisely for large, slow files.
///
/// The shape is `run_import`'s, and so is the rule it turns on: **re-resolve the vault after
/// re-taking the guard.** The work between can take minutes, and if the vault was forgotten or
/// moved meanwhile, writing the note to the path we remember would put it somewhere nothing is
/// watching. The blobs are already in the old path either way, so the refusal says where they are.
fn ingest_unlocked(
    app: &App,
    scope: &Scope,
    vault: &str,
    filename: &str,
    from: IngestSource<'_>,
) -> Result<crate::dto::ObjectMeta, String> {
    // Resolve, then drop the guard: everything below spawns processes.
    let cfg = app.lock()?.config(scope, vault)?;

    let ing = match from {
        IngestSource::Bytes(body) => fm_core::ingest::ingest_bytes(&cfg.path, filename, body),
        IngestSource::Session(session) => {
            let part = fm_core::chunked::assembled(&cfg.path, session).map_err(|e| e.to_string())?;
            fm_core::ingest_file_named(&cfg.path, &part, filename)
        }
    }
    .map_err(|e| e.to_string())?;
    // Best-effort, exactly as `asset_note` treats it — and this is the second subprocess. Doing it
    // here rather than leaving it to `asset_note` is the whole point; `ingest::thumbnail` returns
    // early when the file already exists, so the call inside `asset_note` costs a `stat`.
    let _ = fm_core::ingest::thumbnail(&cfg.path, &ing.hash);

    let mut g = app.lock()?;
    let now = g.config(scope, vault)?;
    if now.path != cfg.path {
        return Err(format!(
            "'{}' moved while the file was being read, so no note was written for it. The bytes \
             are already stored in {}.",
            cfg.name,
            cfg.path.display()
        ));
    }
    let (path, name) = (now.path.clone(), now.name.clone());
    let meta = commands::asset_note(&mut g.store(scope), &path, &name, &ing)
        .map_err(|e| e.to_string())?;
    drop(g);

    if let IngestSource::Session(session) = from {
        // Only on success: a failed ingest leaves the slices where they are so the finish can be
        // retried without re-uploading, and the boot sweep collects them if nobody does.
        let _ = fm_core::chunked::discard(&cfg.path, session);
    }
    Ok(meta)
}

/// Where the bytes for an ingest come from — in memory, or already assembled on disk.
#[derive(Clone, Copy)]
enum IngestSource<'a> {
    Bytes(&'a [u8]),
    Session(&'a str),
}

/// The media tier, for **one** vault — snapshot it into *its own* restic repo.
///
/// Per vault because a restic repo is per repository: there is no single destination
/// that could hold a set of vaults, so each one either has a repo of its own or has
/// nowhere for its media to go. A vault without one is not an error and must not be
/// silently folded into someone else's repo — the caller is told, by name, that this
/// vault's media stayed put. Refusing to say so is the overstatement this whole panel
/// exists to prevent.
fn run_backup(app: &App, scope: &Scope, vault: &str) -> Result<BackupRun, String> {
    // Owned, and the guard dropped: restic can take minutes, and nothing else may write
    // notes meanwhile — but everything else may read them.
    let v = app.lock()?.config(scope, vault)?;
    let repo = v.restic.as_ref().ok_or_else(|| {
        format!(
            "no restic repo configured for '{}' — its media has nowhere to go",
            v.name
        )
    })?;
    let password = crate::secrets::restic_password().ok_or_else(|| {
        "this vault has a media-backup repo but no password, so nothing can be written to it — \
         set one in Backup settings"
            .to_string()
    })?;
    let done = backup::backup(&v.path, Path::new(repo), &password)
        .map_err(|e| format!("backing up '{}': {e}", v.name))?;
    Ok(BackupRun {
        vault: v.name.clone(),
        notes_dir: done.notes_dir,
        blobs: done.blobs,
        contents: done.contents.map(|c| SnapshotContents {
            id: c.id,
            files_new: c.files_new,
            files_changed: c.files_changed,
            files_unmodified: c.files_unmodified,
            bytes_processed: c.bytes_processed,
            bytes_added: c.bytes_added,
        }),
    })
}

/// Everything the form needs to decide, and the verdict itself.
///
/// Takes the vault list rather than reading it, so `create_vault` can re-run the identical
/// check **under the same guard** it then mutates — otherwise two concurrent creates both
/// pass a check and both win.
fn check_path(
    v: &Vaults,
    config: Option<&Path>,
    config_writable: bool,
    name: &str,
    path: &str,
) -> PathCheck {
    let path = PathBuf::from(vaults::expand_home(path));
    let facts = commands::inspect_path(&path);

    // A label, never joined onto a filesystem path, so it stays maximally permissive:
    // `Lab — Ravi's group` must work. Refuse only what actually breaks — empty (an empty
    // name already *means* "the default", so it would make the default unaddressable) and
    // control characters (which would make a JSON entry unreadable to the human who has to
    // fix it).
    let name_ok = !name.trim().is_empty() && !name.chars().any(char::is_control);
    let name_taken = v.list.iter().any(|e| e.name == name);

    // Canonical, because `~/notes` and `/home/you/notes/../notes` are the same directory
    // and only one of them may be a vault.
    let me = vaults::absolute(&path);
    let path_taken = v.list.iter().any(|e| vaults::absolute(&e.path) == me);
    let overlaps = v
        .list
        .iter()
        .find(|e| {
            let theirs = vaults::absolute(&e.path);
            theirs != me && (me.starts_with(&theirs) || theirs.starts_with(&me))
        })
        .map(|e| e.name.clone());

    let ok = name_ok
        && !name_taken
        && !path_taken
        && overlaps.is_none()
        && !facts.not_a_directory
        && facts.writable
        && config_writable
        && config.is_some();

    PathCheck {
        facts,
        name_ok,
        name_taken,
        path_taken,
        overlaps,
        config_writable,
        ok,
    }
}

/// Create a vault, register it, and make it live — no restart.
///
/// **The invariant: nothing is registered in memory until `vaults.json` on disk says the
/// same thing.** Reverse the order and a failed config write leaves an in-memory vault
/// that vanishes on the next start *while the user is capturing notes into it*, which is
/// note loss. So the config write is the commit point, and everything before it is
/// recoverable by trying again.
///
/// Deliberately does **not** call `git::ensure_repo`: `commit_all` already does, on the
/// first auto-commit, gated by `ping.git`. Doing it here would buy an empty `.git` five
/// seconds early — and if the path sits inside a repo the user owns, `ensure_repo` probes
/// only `<path>/.git`, finds none, and `git init`s a **nested repo shadowing theirs**.
fn create_vault(app: &App, name: &str, path: &str) -> Result<Vec<VaultInfo>, String> {
    let mut g = app.lock()?;

    // Re-run the check under the guard. The UI already ran it, but this is TOCTOU
    // territory and curl is a supported client.
    let check = check_path(&g, app.config.as_deref(), app.config_writable, name, path);
    if !check.ok {
        return Err(refusal(&check, name));
    }
    let config = app.config.clone().ok_or(
        "there is nowhere to save the vault list on this machine — set FM_VAULTS".to_string(),
    )?;
    let path = PathBuf::from(vaults::expand_home(path));

    // The directories. `blobs/` and `derived/` eagerly, which needs no git and costs
    // nothing (git cannot track an empty directory anyway) — it just means the ignore
    // lines `ensure_repo` writes on the first commit refer to something the user can see.
    for d in [&path, &path.join("blobs"), &path.join("derived")] {
        std::fs::create_dir_all(d).map_err(|e| format!("could not create {}: {e}", d.display()))?;
    }

    // Open it. On failure we leave the directory alone — it may have pre-existed, and this
    // codebase does not delete user data on a failure path.
    let store = fm_core::FileStore::named(&path, name).map_err(|e| {
        format!(
            "created the directory at {}, but could not open it as a vault: {e} \
             — nothing was configured",
            path.display()
        )
    })?;

    // The commit point.
    let cfg = VaultConfig {
        name: name.to_string(),
        path: path.clone(),
        restic: None,
    };
    let mut list = g.configs();
    list.push(cfg.clone());
    vaults::save(&list, &config).map_err(|e| {
        format!(
            "the vault directory at {} was created, but the vault list could not be \
             saved: {e} — it is not configured. Nothing was lost; fix that and create it again.",
            path.display()
        )
    })?;

    g.add(cfg, store);
    let names = g.all.names();
    Ok(infos(&g.configs(), &names))
}

// ── importing another app's notes ───────────────────────────────────────────────

/// The preview, and **the whole verdict** — source *and* destination in one `ok`.
///
/// Both halves are here for the reason `check_path`'s own comment gives: "duplicating the policy
/// in the browser is how you get a button that enables and then fails". Answering only "is this a
/// Logseq graph?" and leaving the panel to AND that with a separate `check_path` would be exactly
/// the second opinion that rule forbids — so this asks both questions and returns one answer.
#[derive(serde::Serialize)]
struct ImportCheck {
    /// `logseq` / `obsidian`, or absent when the folder is neither.
    format: Option<String>,
    label: Option<String>,
    pages: usize,
    journals: usize,
    attachments: usize,
    #[serde(rename = "attachmentBytes")]
    attachment_bytes: u64,
    #[serde(rename = "leftBehind")]
    left_behind: Vec<commands::LeftBehind>,
    /// One sentence, the most disqualifying first — a list of every complaint at once is how a
    /// user fixes one thing and gets a different refusal. `None` means it can be imported.
    problem: Option<String>,
    ok: bool,
}

/// Why the destination cannot receive an import, if it cannot.
fn destination_problem(
    g: &Vaults,
    app: &App,
    scope: &Scope,
    vault: &str,
    name: &str,
    path: &str,
) -> Option<String> {
    if !vault.is_empty() {
        // An existing vault: it has to be one this caller can actually see.
        return g.config(scope, vault).err();
    }
    let check = check_path(g, app.config.as_deref(), app.config_writable, name, path);
    (!check.ok).then(|| refusal(&check, name))
}

fn import_check(source: &str, destination: Option<String>) -> ImportCheck {
    // The walk happens with the guard released — see the arm. A source path can be anything a
    // person typed, including `/`, so this must never run while holding the vault mutex.
    let scan = fm_core::import::scan(&PathBuf::from(vaults::expand_home(source)));
    let problem = scan.problems.first().cloned().or(destination);
    let ok = problem.is_none() && scan.ok();
    ImportCheck {
        format: scan.format.map(|f| f.as_str().to_string()),
        label: scan.format.map(|f| f.label().to_string()),
        pages: scan.pages,
        journals: scan.journals,
        attachments: scan.attachments,
        attachment_bytes: scan.attachment_bytes,
        left_behind: scan
            .left_behind
            .into_iter()
            .map(|(kind, count)| commands::LeftBehind { kind, count })
            .collect(),
        problem,
        ok,
    }
}

/// Convert a Logseq graph or an Obsidian vault into notes.
///
/// # Where the lock is, and why it is there and not elsewhere
///
/// Converting walks a directory, hashes every attachment and runs `pdftotext` over every PDF.
/// `papers-plan.md` B5 records what happens when that kind of work runs under the app's mutex:
/// *"Bulk ingest holds the global app lock across two subprocess spawns per file … A 5,000-PDF
/// import freezes every tab for 25–40 minutes."* So the guard is taken three times, briefly, and
/// **released across the conversion** — the pattern `adding-features.md` states: "clone what you
/// need out and drop the guard first".
///
/// The window that buys is real, so the destination is **re-resolved** after it. In between,
/// another client can forget the vault or a second import can take the name.
fn run_import(
    app: &App,
    scope: &Scope,
    source: &str,
    vault: &str,
    name: &str,
    path: &str,
    stubs: bool,
) -> Result<commands::ImportReport, String> {
    let source = PathBuf::from(vaults::expand_home(source));
    let opts = fm_core::import::Options { create_stubs: stubs };

    if vault.is_empty() {
        return import_into_new_vault(app, &source, name, path, opts);
    }

    // ── an existing vault ──────────────────────────────────────────────────────
    // What has already been imported here, in one pass. Under the guard because it reads the
    // store; it is a single scan, which is the point (see `existing_sources`).
    let (cfg, existing) = {
        let mut g = app.lock()?;
        let cfg = g.config(scope, vault)?;
        let st = g.store(scope);
        let existing = commands::existing_sources(&st).map_err(err)?;
        (cfg, existing)
    };

    let converted =
        fm_core::import::convert(&source, &cfg.path, &existing, opts).map_err(err)?;

    let mut g = app.lock()?;
    // **Re-resolved, not reused.** The conversion above can take minutes; if the vault was
    // forgotten or moved in that time, writing to the path we remember would put notes somewhere
    // nothing is watching. The blobs are already in the old path either way — say so.
    let now = g.config(scope, vault)?;
    if now.path != cfg.path {
        return Err(format!(
            "'{vault}' moved while the import was running, so nothing was written to it. Any              attachments already copied are in {}.",
            cfg.path.display()
        ));
    }
    let mut report = {
        let mut st = g.store(scope);
        commands::write_import(&mut st, &now.path, &now.name, converted).map_err(err)?
    };
    record(&mut g, &now.name, &now.path, &mut report);
    Ok(report)
}

/// Import into a vault that does not exist yet — a fourth way one comes into being, beside
/// create, clone and restore. Same ordering as [`create_vault`], with the conversion moved
/// outside the guard.
fn import_into_new_vault(
    app: &App,
    source: &Path,
    name: &str,
    path: &str,
    opts: fm_core::import::Options,
) -> Result<commands::ImportReport, String> {
    // 1. Validate and make the directory, under the guard.
    let (mut store, path, config) = {
        let g = app.lock()?;
        let check = check_path(&g, app.config.as_deref(), app.config_writable, name, path);
        if !check.ok {
            return Err(refusal(&check, name));
        }
        let config = app.config.clone().ok_or(
            "there is nowhere to save the vault list on this machine — set FM_VAULTS".to_string(),
        )?;
        let path = PathBuf::from(vaults::expand_home(path));
        for d in [&path, &path.join("blobs"), &path.join("derived")] {
            std::fs::create_dir_all(d)
                .map_err(|e| format!("could not create {}: {e}", d.display()))?;
        }
        // Deliberately no `acquire::naturalise`: nothing *arrived* here. Its own contract is
        // "only ever call this on something that just arrived", and `create_vault` does not call
        // it either — the directory we just made has no foreign index and no sender's identity.
        let store = fm_core::FileStore::named(&path, name).map_err(|e| {
            format!(
                "created the directory at {}, but could not open it as a vault: {e}                  — nothing was configured",
                path.display()
            )
        })?;
        (store, path, config)
    };

    // 2. The long part, with the guard released and into a store nobody else can see yet.
    let converted = fm_core::import::convert(source, &path, &HashMap::new(), opts).map_err(err)?;
    let mut report = commands::write_import(&mut store, &path, name, converted).map_err(err)?;

    // 3. Register it. The name may have been taken while we were converting, so the check is
    //    re-run rather than assumed — and `vaults::save` is still the commit point: JSON before
    //    memory, so a failed write never leaves a vault that vanishes on restart.
    let mut g = app.lock()?;
    let check = check_path(&g, app.config.as_deref(), app.config_writable, name, &path.to_string_lossy());
    if !check.ok {
        return Err(format!(
            "the notes were imported into {}, but the vault could not be registered: {}.              Nothing was lost — the folder is there; add it with New vault once that is fixed.",
            path.display(),
            refusal(&check, name)
        ));
    }
    let cfg = VaultConfig { name: name.to_string(), path: path.clone(), restic: None };
    let mut list = g.configs();
    list.push(cfg.clone());
    vaults::save(&list, &config).map_err(|e| {
        format!(
            "the notes were imported into {}, but the vault list could not be saved: {e}              — it is not configured. Nothing was lost; fix that and add it with New vault.",
            path.display()
        )
    })?;
    g.add(cfg, store);
    record(&mut g, name, &path, &mut report);
    Ok(report)
}

/// Put the whole import into history as **one** entry, which is what makes it undoable as one.
///
/// Failure here is not failure of the import: the notes are on disk and readable either way, so
/// this reports rather than returns an error. That matters at size — `commit_all` hands every
/// path to `git` in one argv, and a very large graph can exceed what the OS will accept.
fn record(g: &mut Vaults, name: &str, path: &Path, report: &mut commands::ImportReport) {
    if !vcs::available() {
        return;
    }
    let paths = g.all.written(name);
    let message = format!("import: {} notes from {}", report.notes, report.format);
    match vcs::commit_all(path, &message, &paths) {
        Ok(true) => {
            g.all.clear_written(name);
            report.recorded = true;
        }
        Ok(false) => {}
        Err(e) => report.warnings.push(format!(
            "the notes are all in the vault, but they could not be recorded in one step ({e}) —              open Back up and record them there"
        )),
    }
}

/// Unregister a vault: drop it from the live set and from `vaults.json`.
///
/// **It never deletes a file, and that is the whole safety argument.** "Forget" and "destroy" are
/// different verbs, and only one of them is reversible: a vault removed from the list can be added
/// back by pointing at the same directory, so the worst case of a mistaken click is retyping a path.
/// Deleting notes would make the worst case unbounded — and this codebase does not delete user data
/// on any path, including the ones the user asked for. The answer therefore *says* where the files
/// still are, so nobody is left wondering whether they went.
///
/// Removing the last vault is allowed. It lands on the first-run screen, which is the honest state
/// for a machine with no vaults, and `list_vaults` returning `[]` is exactly how the UI already
/// detects it.
fn forget_vault(app: &App, name: &str) -> Result<serde_json::Value, String> {
    if name.is_empty() {
        return Err("which vault? forget_vault needs a name".into());
    }
    let mut g = app.lock()?;
    let cfg = g
        .configs()
        .into_iter()
        .find(|c| c.name == name)
        .ok_or_else(|| format!("no vault named '{name}'"))?;
    let config = app.config.clone().ok_or(
        "there is nowhere to save the vault list on this machine, so it cannot be changed"
            .to_string(),
    )?;
    // What is being left behind, counted *before* the vault leaves the live set — afterwards there
    // is nothing to ask. This is the number the UI shows so "removed" is never mistaken for "erased".
    let notes = std::fs::read_dir(
        fm_core::descriptor::Descriptor::read(&cfg.path)
            .map(|d| d.notes_dir(&cfg.path))
            .unwrap_or_else(|_| cfg.path.join("notes")),
    )
    .map(|rd| {
        rd.flatten()
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("md"))
            .count()
    })
    .unwrap_or(0);
    let remote = vcs::remote(&cfg.path).ok().flatten();

    // The list is saved **first**: if that fails, nothing has changed and the vault is still there,
    // which is the recoverable order. Dropping it from memory first would leave a running app whose
    // vault list disagrees with the file it will reload from.
    let remaining: Vec<VaultConfig> = g.configs().into_iter().filter(|c| c.name != name).collect();
    // **Two writers, in this order, and neither alone is enough.**
    //
    // `save` materialises: an `FM_VAULT`-only vault that is not in the file yet must be written
    // before the file has entries, because `load` prefers the file over `FM_VAULT` the moment it
    // does — the same reason `set_restic` documents for calling `save` first. It is given
    // `remaining`, so it cannot re-add the vault being forgotten.
    //
    // `forget` removes. `save` **cannot**: it appends only and skips names already on disk, so the
    // call below used to be the whole of this and removed nothing at all — the entry stayed in
    // `vaults.json` and the vault returned on the next start.
    vaults::save(&remaining, &config)
        .map_err(|e| format!("the vault list could not be saved: {e} — nothing was changed"))?;
    // **Not "nothing was changed"**: `save` above may already have materialised entries, so this
    // message must not inherit that claim. The vault is still usable either way — nothing here
    // touches its files.
    vaults::forget(name, &config).map_err(|e| {
        format!("'{name}' could not be removed from the vault list: {e} — it is still listed")
    })?;
    g.remove(name);
    let names = g.all.names();
    Ok(serde_json::json!({
        "forgotten": name,
        "path": cfg.path.to_string_lossy(),
        "notes": notes,
        "remote": remote,
        "vaults": infos(&g.configs(), &names),
    }))
}

/// Clone a collaborator's vault and register it — the other way a vault comes into being.
///
/// Same shape as [`create_vault`] with two additions, and the ordering between them is the
/// whole design:
///
/// **The identity is validated before anything is fetched.** A clone that succeeds and then
/// fails on a mistyped email would leave a real repo on disk that is not a registered vault,
/// in a directory the user cannot retry into because it is no longer empty. Since the
/// validation is pure, it costs nothing to do first — so a bad identity refuses the whole
/// operation while the disk is still untouched.
///
/// **The identity is required, not optional.** A cloned vault has an audience by definition,
/// which is exactly the case where committing as the placeholder attributes everyone's work
/// to one fake person (`decisions.md`: a vault gains an identity when it gains an audience).
/// `create_vault` can reasonably leave it to the placeholder; this cannot.
fn clone_vault(
    app: &App,
    name: &str,
    path: &str,
    url: &str,
    git_name: &str,
    git_email: &str,
) -> Result<Vec<VaultInfo>, String> {
    let mut g = app.lock()?;

    let check = check_path(&g, app.config.as_deref(), app.config_writable, name, path);
    if !check.ok {
        return Err(refusal(&check, name));
    }
    if url.trim().is_empty() {
        return Err("a shared vault needs the URL of the repo to clone".into());
    }
    // Pre-flight, mirroring `git::set_identity`'s own rules, so the failure lands here rather
    // than after a clone has already written to disk.
    if git_name.trim().is_empty() || git_email.trim().is_empty() {
        return Err(
            "a shared vault needs your name and email — they sign every commit you \
                    make in it, and your collaborators see them"
                .into(),
        );
    }
    if !git_email.contains('@') {
        return Err(format!("'{git_email}' is not an email address"));
    }
    let config = app.config.clone().ok_or(
        "there is nowhere to save the vault list on this machine — set FM_VAULTS".to_string(),
    )?;
    let path = PathBuf::from(vaults::expand_home(path));

    fm_core::vcs::clone(url, &path).map_err(|e| format!("could not clone {}: {e}", url.trim()))?;

    // Every way of acquiring a vault goes through the same step, and it runs **before** the
    // identity is set: `naturalise` forgets any committer that arrived with the directory,
    // which is a no-op for a clone (git declines to carry `.git/config`) and essential for
    // any transport that moves the directory whole. Doing it after `set_identity` would
    // erase the identity we just asked the user for.
    fm_core::acquire::naturalise(&path).map_err(|e| {
        format!(
            "cloned into {}, but could not prepare it for this machine: {e} — the clone is \
             on disk and intact; nothing was configured",
            path.display()
        )
    })?;

    fm_core::vcs::set_identity(&path, git_name, git_email).map_err(|e| {
        format!(
            "cloned into {}, but could not set who you are in it: {e} — the clone is on disk \
             and intact; nothing was configured",
            path.display()
        )
    })?;

    let store = fm_core::FileStore::named(&path, name).map_err(|e| {
        format!(
            "cloned into {}, but could not open it as a vault: {e} — the clone is on disk \
             and intact; nothing was configured",
            path.display()
        )
    })?;

    let cfg = VaultConfig {
        name: name.to_string(),
        path: path.clone(),
        restic: None,
    };
    let mut list = g.configs();
    list.push(cfg.clone());
    vaults::save(&list, &config).map_err(|e| {
        format!(
            "the vault was cloned into {}, but the vault list could not be saved: {e} — it is \
             not configured. Nothing was lost; fix that and add it again.",
            path.display()
        )
    })?;

    g.add(cfg, store);
    let names = g.all.names();
    Ok(infos(&g.configs(), &names))
}

/// Restore a vault from a restic repo and register it — the third way a vault comes into
/// being, and the one for a machine that is not the machine the vault was on.
///
/// Same shape as [`clone_vault`]: validate everything cheap and pure *first*, so a mistyped
/// field refuses while the disk is untouched, and only then touch the network.
///
/// **What you get back is notes and media, with no history.** [`fm_core::backup::backup`]
/// snapshots the vault's own directories and deliberately not its root, so `.git` was never
/// in the repo to come back — which also means no remote, no collaborators, and no identity.
/// That is the honest shape of a backup as an acquisition method and the UI says so in as
/// many words. It is a *recovery*, not a *join*: the vault that arrives is a first-class
/// vault, and if the user later wants collaboration they turn on history and add a remote,
/// exactly as they would for a vault they had made locally.
///
/// **No identity is required, and that is the difference from a clone.** A clone has an
/// audience by definition; a restore has one user by definition — theirs. Demanding a name
/// and email to recover your own notes would be ceremony.
///
/// The password comes from `RESTIC_PASSWORD` and is never taken as an argument, stored, or
/// echoed: the app holds no secret of its own, and a restore is not the place to start.
fn restore_vault(app: &App, name: &str, path: &str, repo: &str) -> Result<Vec<VaultInfo>, String> {
    let mut g = app.lock()?;

    let check = check_path(&g, app.config.as_deref(), app.config_writable, name, path);
    if !check.ok {
        return Err(refusal(&check, name));
    }
    if repo.trim().is_empty() {
        return Err("restoring needs the restic repository the backup is in".into());
    }
    // Ask before doing, so "restic isn't installed" is not reported as a failed restore.
    if !backup::available() {
        return Err(
            "restic is not installed on this machine, so there is no backup to \
                    restore from — a vault can still be created here, or cloned with git"
                .into(),
        );
    }
    // **Points at the field that exists.** This used to say the app never stores a password and
    // send the reader to an environment variable; since 2026-08-29 it keeps one, `0600`, and the
    // panel asks for it. Advice naming a mechanism the product no longer uses is worse than none.
    let password = crate::secrets::restic_password().ok_or(
        "set the media-backup password in Backup settings — it is what unlocks the repository",
    )?;
    let config = app.config.clone().ok_or(
        "there is nowhere to save the vault list on this machine — set FM_VAULTS".to_string(),
    )?;
    let path = PathBuf::from(vaults::expand_home(path));
    let repo = PathBuf::from(vaults::expand_home(repo));

    std::fs::create_dir_all(&path)
        .map_err(|e| format!("could not create {}: {e}", path.display()))?;

    let restored = fm_core::backup::restore_vault(&repo, &password, &path)
        .map_err(|e| format!("could not restore from {}: {e}", repo.display()))?;

    // The same step every acquisition shares. Here it is mostly the directories — a restic
    // snapshot carries neither `.git` nor an index — but routing through it is what keeps
    // "add a transport" from meaning "reimplement the safety".
    fm_core::acquire::naturalise(&path).map_err(|e| {
        format!(
            "restored into {}, but could not prepare it for this machine: {e} — the files \
             are on disk and intact; nothing was configured",
            path.display()
        )
    })?;

    // A `vault.json` is not in the snapshot (it lives at the vault root, which `backup` does
    // not take), so a vault whose notes were in `docs/` would restore its notes and then be
    // opened looking in `notes/` — every note invisible, and nothing to say why. The
    // snapshot's own recorded paths are the only surviving record of that name, so write the
    // descriptor back from them. Skipped when it is already the default.
    if restored.notes_dir != "notes" {
        let d = fm_core::descriptor::Descriptor {
            notes: Some(PathBuf::from(&restored.notes_dir)),
            ..Default::default()
        };
        d.write_new(&path).map_err(|e| {
            format!(
                "restored into {}, but could not record that its notes are in {}/: {e}",
                path.display(),
                restored.notes_dir
            )
        })?;
    }

    let store = fm_core::FileStore::named(&path, name).map_err(|e| {
        format!(
            "restored into {}, but could not open it as a vault: {e} — the files are on disk \
             and intact; nothing was configured",
            path.display()
        )
    })?;

    // Remember where it came from. A restored vault has no remote and no history, so its
    // restic repo is the only thing connecting it to anywhere — and the user who just typed
    // it should not have to type it again to back up.
    let cfg = VaultConfig {
        name: name.to_string(),
        path: path.clone(),
        restic: Some(repo.to_string_lossy().into_owned()),
    };
    let mut list = g.configs();
    list.push(cfg.clone());
    vaults::save(&list, &config).map_err(|e| {
        format!(
            "the vault was restored into {}, but the vault list could not be saved: {e} — it \
             is not configured. Nothing was lost; fix that and add it again.",
            path.display()
        )
    })?;

    g.add(cfg, store);
    let names = g.all.names();
    Ok(infos(&g.configs(), &names))
}

/// Where this machine's git credentials live, and whether it has one for a given URL.
#[derive(serde::Serialize)]
struct GitAuth {
    /// `system` — git's credential helper owns it, and we store nothing.
    /// `app`    — no helper exists here, so formicaria keeps the token itself.
    /// `none`   — no git at all; nothing to authenticate with.
    storage: &'static str,
    /// A credential is available for this URL right now. On the desktop this asks the helper,
    /// which is how "it was already set up, like in this repo" answers itself without the user
    /// having to know whether it was.
    have_credential: bool,
    /// Only meaningful for `system`: the helper's name, whether it is plaintext, and a better
    /// one if this machine has it installed.
    helper: Option<fm_core::git::HelperAdvice>,
}

/// The auth situation, as the form needs it.
fn git_auth(url: &str) -> GitAuth {
    if !fm_core::vcs::available() {
        return GitAuth {
            storage: "none",
            have_credential: false,
            helper: None,
        };
    }
    if fm_core::git::available() {
        GitAuth {
            storage: "system",
            have_credential: fm_core::git::credential_exists(url),
            helper: Some(fm_core::git::helper_advice()),
        }
    } else {
        // A phone. There is no helper to ask, so the only question is whether we hold a token —
        // and it is per device, not per URL, because one account per method per device is the
        // stance this design took.
        GitAuth {
            storage: "app",
            have_credential: crate::secrets::has_token(),
            helper: None,
        }
    }
}

/// What the new-vault form shows under a repo URL: can we reach it, and if not, what to do.
#[derive(serde::Serialize)]
struct RemoteProbe {
    /// `reachable` | `needs_auth` | `unreachable` — the shape the UI branches on.
    state: &'static str,
    /// One sentence naming what to do next. Never git's raw text for the two cases we
    /// understand, always git's raw text for the one we do not.
    detail: String,
    /// The credential helper configured on this machine, or `null`. **A program name, never a
    /// secret.** Present so the advice can be specific instead of a link to a manual.
    helper: Option<String>,
    /// `true` when the configured helper keeps credentials in **plaintext**. Worth saying
    /// unprompted: most people running `store` were told to by a tutorial and have no idea
    /// their token is sitting in `~/.git-credentials` in the clear.
    helper_is_plaintext: bool,
}

/// Ask a remote whether we could clone it, and turn the answer into advice.
///
/// **Never fails.** Every outcome — including "there is no git here" — is a state the form
/// renders, because this runs while the user is still typing and an error banner on every
/// keystroke of a half-typed URL would be worse than useless.
fn probe_remote(url: &str) -> RemoteProbe {
    let helper = fm_core::git::credential_helper();
    // `store` is the one that matters: git writes `~/.git-credentials` unencrypted. `cache` is
    // memory-only and fine; the platform keychains are fine.
    let helper_is_plaintext = helper.as_deref().is_some_and(|h| h == "store");
    let advise = |state, detail: String| RemoteProbe {
        state,
        detail,
        helper: helper.clone(),
        helper_is_plaintext,
    };

    if url.trim().is_empty() {
        return advise("unreachable", String::new());
    }
    match fm_core::vcs::probe(url) {
        fm_core::git::Probe::Reachable => {
            advise("reachable", "This repo answered — you can clone it.".into())
        }
        fm_core::git::Probe::NeedsAuth => {
            // The advice is per platform because the *fix* is per platform, and a generic
            // "configure your credentials" is what sends people to a search engine.
            let detail = if fm_core::git::available() {
                match helper.as_deref() {
                    // A helper is configured and we still could not read it: the stored
                    // credential is missing for this host or no longer valid. That is a
                    // different problem from having no helper, and saying so saves an hour.
                    Some(h) => format!(
                        "This repo needs credentials. Git is set up to use the '{h}' helper on                          this machine, but it had nothing valid for this host — the token may                          have expired, or never been saved for it. Authenticate once in a                          terminal (`git ls-remote <url>`) and the helper will remember."
                    ),
                    None => "This repo needs credentials and git has no credential helper                              configured on this machine. Either use an SSH URL (git@…) with a                              key in your agent, or set a helper — `git config --global                              credential.helper` — then authenticate once in a terminal."
                        .into(),
                }
            } else {
                // No git binary: this is the phone. There is no helper to configure and no
                // terminal to authenticate in, so the app has to hold a token itself.
                //
                // **Names where the field actually is.** An earlier draft sent people to
                // Settings, which does not have it — the token input sits directly below this
                // message in the clone form. Advice pointing at the wrong screen is worse than
                // none, because it reads as authoritative.
                "This repo needs credentials. Paste a personal access token below — this \
                 device has no system-wide git configuration to fall back on, so formicaria \
                 keeps it in its own private storage."
                    .into()
            };
            advise("needs_auth", detail)
        }
        // Deliberately git's own words. We did not recognise this, and inventing a friendlier
        // sentence would mean guessing — which is how someone ends up configuring credentials
        // for a URL they simply mistyped.
        fm_core::git::Probe::Unreachable(why) => advise("unreachable", why),
    }
}

/// Where a vault goes, given what the caller asked for.
///
/// **An empty `path` means "you decide", and only a managed installation may decide.** On a
/// phone the form never asks for a folder — there is no path a user could meaningfully type —
/// so it sends the name alone and this resolves it inside `FM_VAULT_ROOT`. On a desktop there
/// is no root, an empty path stays empty, and `check_path` refuses it the way it always has.
///
/// **Resolved here rather than in the UI on purpose.** A browser joining `<root>/<name>` is one
/// `../` away from writing outside the sandbox, and containment that depends on the frontend
/// behaving is not containment. A caller may still name an explicit path — `curl` is a
/// supported client — and on a phone that path is checked by `check_path` like any other.
fn resolve_path(name: &str, path: &str) -> Result<String, String> {
    if !path.trim().is_empty() {
        return Ok(path.to_string());
    }
    match vaults::vault_root() {
        Some(root) => Ok(vaults::contained_path(&root, name)?
            .to_string_lossy()
            .into_owned()),
        None => Ok(String::new()),
    }
}

/// Why the form said no. One sentence, the most disqualifying first — a list of every
/// complaint at once is how a user fixes one thing and gets a different refusal.
fn refusal(c: &PathCheck, name: &str) -> String {
    if !c.config_writable {
        return "the vault list on disk isn't something we can safely write — fix it first, \
                then try again"
            .into();
    }
    if !c.name_ok {
        return "a vault needs a name".into();
    }
    if c.name_taken {
        return format!("there is already a vault called '{name}'");
    }
    if c.path_taken {
        return "that folder is already a vault".into();
    }
    if let Some(other) = &c.overlaps {
        return format!(
            "that folder is inside '{other}' (or contains it) — one folder cannot be in two \
             audiences at once"
        );
    }
    if c.facts.not_a_directory {
        return "that is a file, not a folder".into();
    }
    if !c.facts.writable {
        return format!("we cannot write to {}", c.facts.path);
    }
    "there is nowhere to save the vault list on this machine — set FM_VAULTS".into()
}

/// The vault list as the UI needs it.
///
/// Takes the **store's** names alongside the configured ones, because a vault may name
/// itself: a `vault.json` in a repo you cloned supplies the audience label when the local
/// vault list has none to give. Config still wins when it has an opinion — that name is the
/// one *this* user chose, and a repo must not rename their audience out from under them —
/// so this only fills a blank. Without it, adopting a repo shows a vault called "".
fn infos(v: &[VaultConfig], store_names: &[&str]) -> Vec<VaultInfo> {
    // Derived first for the whole list, because the collision rule needs to see all of them: two
    // vaults cloned from one repo would otherwise show the same label, and an ambiguous vault filter
    // hides notes from the wrong vault. When a label is not unique, **both** fall back to their local
    // names — a duplicate label is worse than a local one.
    let derived: Vec<Option<String>> = v.iter().map(|e| remote_label(&e.path)).collect();
    // Read once per vault and reuse. Best-effort: a vault whose descriptor will not parse still
    // belongs in the list and reports the defaults — `Descriptor::read` is where a malformed file is
    // loudly an error; the vault *list* must not fail to render because one vault has a typo.
    let desc: Vec<Option<fm_core::descriptor::Descriptor>> =
        v.iter().map(|e| fm_core::descriptor::Descriptor::read(&e.path).ok()).collect();
    let labels: Vec<Option<String>> = derived
        .iter()
        .map(|d| {
            let l = d.as_ref()?;
            (derived
                .iter()
                .filter(|o| o.as_deref() == Some(l.as_str()))
                .count()
                == 1)
                .then(|| l.clone())
        })
        .collect();
    v.iter()
        .enumerate()
        .map(|(i, e)| VaultInfo {
            label: labels.get(i).cloned().flatten(),
            name: if e.name.is_empty() {
                store_names
                    .get(i)
                    .map(|n| n.to_string())
                    .unwrap_or_default()
            } else {
                e.name.clone()
            },
            path: e.path.to_string_lossy().into_owned(),
            default: i == 0,
            // Best-effort: a vault whose descriptor will not parse still belongs in the list, and
            // reports "off" — the same as having no opinion. `Descriptor::read` is where a
            // malformed file is loudly an error; this call is the vault *list*, which must not
            // fail to render because one vault has a typo in a setting.
            git_assets_max: desc.get(i).and_then(|d| d.as_ref().and_then(|d| d.git_assets_max)),
            supervision: {
                let sup = desc
                    .get(i)
                    .and_then(|d| d.as_ref().map(|d| d.supervision))
                    .unwrap_or_default();
                Supervision { collect: sup.collect, publish: sup.publish }
            },
            // Local `git config` read, beside the `remote_label` spawn above — never a network call.
            identity: vcs::identity(&e.path),
        })
        .collect()
}

/// **The first tests this module has ever had** (2026-09-04).
///
/// `dispatch.rs` is the one command surface — every frontend goes through it — and it contained
/// **zero** `#[test]`. The layer below is well covered (`fm-core`'s eight real-`restic` tests)
/// and the layer above is covered by mocks; the seam between them was where nothing looked.
/// These start with the backup arms, which is where `outstanding.md` §2.10 says the blindness
/// costs most: nothing asserted `backup_status`'s shape, and nothing drove the `backup` arm's
/// refusals.
///
/// **Most of these need nothing installed.** The refusals and the reported shape are exactly the
/// paths that run on a machine which has *not* got everything set up — the machine most likely to
/// be told something wrong — so they are the ones that must never be skipped. Three do need
/// restic (the two restore arms and the snapshot's contents), and each says so and returns rather
/// than failing, because what they prove cannot be proved without it.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::vaults::VaultConfig;
    use fm_core::MultiStore;

    /// **`FM_CONFIG_DIR`, `XDG_CONFIG_HOME` and the stored password are process-global.** Cargo
    /// runs these on many threads in one process, so without this they race: one test clears the
    /// password while another is asserting it is set. Same reason and same shape as
    /// `secrets.rs`'s own `ENV` lock and `backup_records_everything.rs`'s.
    /// `unwrap_or_else(|e| e.into_inner())` so one panic does not poison the mutex and turn a
    /// single failure into every failure.
    static ENV: std::sync::Mutex<()> = std::sync::Mutex::new(());

    struct NoHost;
    impl Host for NoHost {
        fn open_external(&self, _p: &Path) -> Result<(), String> {
            Err("not in a test".into())
        }
    }

    /// An app with one real vault directory, its config isolated in a tempdir.
    ///
    /// Returns the `TempDir`s so the caller holds them: dropping one deletes the directory the
    /// app is pointing at, which produces failures that look like bugs in the code under test.
    fn app_with_vault(restic: Option<&str>) -> (tempfile::TempDir, tempfile::TempDir, App) {
        let home = tempfile::tempdir().unwrap();
        let vault = tempfile::tempdir().unwrap();
        std::env::set_var("FM_CONFIG_DIR", home.path());
        std::env::set_var("XDG_CONFIG_HOME", home.path());
        let list = vec![VaultConfig {
            name: "notes".into(),
            path: vault.path().to_path_buf(),
            restic: restic.map(str::to_string),
        }];
        let store =
            MultiStore::open(&[("notes".to_string(), vault.path().to_path_buf())]).unwrap();
        let app = App::new(store, list, Some(home.path().join("vaults.json")), true);
        (home, vault, app)
    }

    fn call(app: &App, cmd: &str, args: serde_json::Value) -> Result<String, String> {
        dispatch(cmd, &args, &[], app, &NoHost)
            .map(|o| String::from_utf8(o.into_bytes()).unwrap())
    }

    /// **The password is reported as a bool and never by value**, and the three conditions behind
    /// `restic_ready` are reported separately so a panel can name which one is missing.
    ///
    /// The value assertion is the one that matters and it is deliberately a search of the whole
    /// serialized response, not of one field: a leak would arrive as a *new* field somebody added
    /// without thinking, and a test that only checks the fields it knows about cannot see that.
    #[test]
    fn backup_status_reports_the_password_as_a_bool_and_never_its_value() {
        let _guard = ENV.lock().unwrap_or_else(|e| e.into_inner());
        let secret = "correct horse battery staple";
        let (_home, _vault, app) = app_with_vault(Some("/tmp/no-such-restic-repo"));
        crate::secrets::save_restic_password(secret).unwrap();

        let out = call(&app, "backup_status", serde_json::json!({})).unwrap();
        assert!(
            !out.contains(secret),
            "the restic password must never cross the wire: {out}"
        );
        assert!(out.contains("\"restic_password_set\":true"), "{out}");
        // Reported per vault, and separately from the machine-wide facts.
        assert!(out.contains("\"restic_repo\":\"/tmp/no-such-restic-repo\""), "{out}");

        crate::secrets::clear_restic_password().unwrap();
        let out = call(&app, "backup_status", serde_json::json!({})).unwrap();
        assert!(out.contains("\"restic_password_set\":false"), "{out}");
        // **All three conditions, so "ready" means "will work".** With the password gone it must
        // be false even though restic may well be installed and the repo is still configured.
        assert!(out.contains("\"restic_ready\":false"), "{out}");
    }

    /// The `backup` arm's two refusals, which had never been driven.
    ///
    /// Both must name the vault and say what is missing: a backup that declines silently, or with
    /// a generic message, is one somebody assumes ran.
    #[test]
    fn backup_refuses_without_a_repo_and_without_a_password() {
        let _guard = ENV.lock().unwrap_or_else(|e| e.into_inner());

        // No repo configured for this vault.
        let (_home, _vault, app) = app_with_vault(None);
        crate::secrets::save_restic_password("irrelevant here").unwrap();
        let e = call(&app, "backup", serde_json::json!({ "vault": "notes" })).unwrap_err();
        assert!(e.contains("no restic repo configured"), "{e}");
        assert!(e.contains("notes"), "the refusal must name the vault: {e}");

        // A repo, but no password to open it with.
        crate::secrets::clear_restic_password().unwrap();
        let (_home2, _vault2, app) = app_with_vault(Some("/tmp/no-such-restic-repo"));
        let e = call(&app, "backup", serde_json::json!({ "vault": "notes" })).unwrap_err();
        assert!(e.contains("no password"), "{e}");
    }

    /// **A snapshot that crosses the wire saying what it held.**
    ///
    /// The arm answered `nothing()` until 2026-09-05 — `outstanding.md` §2.10's last residue —
    /// so a frontend could learn that a backup had happened and nothing about what was in it.
    /// The panel filled the hole with a fixed phrase, *"notes and attachments"*, which is wrong
    /// for the ordinary vault that has no attachments yet.
    ///
    /// What only this layer can prove: the answer reaches a caller as JSON, keyed by vault, with
    /// `contents` **nested** rather than flattened. The nesting is the point — one nullable field
    /// says *"restic did not describe it"* for the whole summary, where six nullable numbers
    /// would put a caller back to guessing which zero was a zero.
    ///
    /// Proven red by restoring `nothing()` in the arm: the response is `null`, and every
    /// assertion below has nothing to read.
    #[test]
    fn the_backup_arm_says_what_the_snapshot_contained() {
        let _guard = ENV.lock().unwrap_or_else(|e| e.into_inner());
        if !backup::available() {
            eprintln!("skipping: restic is not installed, so no snapshot can be taken");
            return;
        }
        let _cache = ResticCache::scoped();
        let repo = tempfile::tempdir().unwrap();

        let (_home, vault, app) = app_with_vault(Some(repo.path().to_str().unwrap()));
        crate::secrets::save_restic_password("correct horse battery staple").unwrap();

        // One note and deliberately **no `blobs/`** — the case the fixed phrase overstated.
        let notes = vault.path().join("notes");
        std::fs::create_dir_all(&notes).unwrap();
        std::fs::write(notes.join("01.md"), "---\ntype: note\n---\nthe durable knowledge\n")
            .unwrap();

        let out = call(&app, "backup", serde_json::json!({ "vault": "notes" }))
            .expect("the snapshot must succeed");
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();

        assert_eq!(v["vault"], "notes", "keyed by vault, as the panel runs it: {out}");
        assert_eq!(v["notes_dir"], "notes", "{out}");
        assert_eq!(v["blobs"], false, "this vault has no attachments: {out}");

        let c = &v["contents"];
        assert!(c.is_object(), "restic described the snapshot, so this is not null: {out}");
        assert_eq!(
            c["id"].as_str().map(str::len),
            Some(8),
            "the short id, so this and `backup_latest` name the same snapshot: {out}"
        );
        assert_eq!(c["files_new"].as_u64(), Some(1), "the one note: {out}");
        assert!(c["bytes_added"].as_u64().unwrap_or(0) > 0, "{out}");

        crate::secrets::clear_restic_password().unwrap();
    }

    /// A vault that is not ours is refused **before** anything is written.
    ///
    /// The scope check is what stops a paired device reconfiguring an audience it was never
    /// given, and `set_restic_repo` writes the vault list — so the order matters, not just the
    /// verdict.
    #[test]
    fn set_restic_repo_refuses_a_vault_that_is_not_there() {
        let _guard = ENV.lock().unwrap_or_else(|e| e.into_inner());
        let (home, _vault, app) = app_with_vault(None);
        let list = home.path().join("vaults.json");

        let e = call(
            &app,
            "set_restic_repo",
            serde_json::json!({ "vault": "not-a-vault", "repo": "/tmp/x" }),
        )
        .unwrap_err();
        assert!(!e.is_empty(), "an unknown vault must be refused");
        assert!(
            !list.exists(),
            "the refusal must come before the vault list is written"
        );
    }

    /// Setting a repo and clearing it both round-trip through the file, not just through memory.
    ///
    /// The distinction is the whole point: an in-memory-only write looks identical until the app
    /// restarts, which is the moment somebody discovers their backups were never configured.
    #[test]
    fn set_restic_repo_round_trips_and_an_empty_value_clears_it() {
        let _guard = ENV.lock().unwrap_or_else(|e| e.into_inner());
        let (home, _vault, app) = app_with_vault(None);

        let out = call(
            &app,
            "set_restic_repo",
            serde_json::json!({ "vault": "notes", "repo": "/tmp/lab-backup" }),
        )
        .unwrap();
        assert!(out.contains("/tmp/lab-backup"), "{out}");
        let on_disk = std::fs::read_to_string(home.path().join("vaults.json")).unwrap();
        assert!(on_disk.contains("/tmp/lab-backup"), "not persisted: {on_disk}");

        let out = call(
            &app,
            "set_restic_repo",
            serde_json::json!({ "vault": "notes", "repo": "" }),
        )
        .unwrap();
        assert!(out.contains("\"restic_repo\":null"), "empty must clear it: {out}");
        let on_disk = std::fs::read_to_string(home.path().join("vaults.json")).unwrap();
        assert!(!on_disk.contains("/tmp/lab-backup"), "still on disk: {on_disk}");
    }

    /// **"Never backed up" and "this machine cannot tell you" are different answers**, and
    /// `backup_latest` has to keep them apart — that separation is the reason the command exists
    /// in the shape it does.
    #[test]
    fn backup_latest_says_why_when_it_cannot_answer() {
        let _guard = ENV.lock().unwrap_or_else(|e| e.into_inner());
        crate::secrets::clear_restic_password().unwrap();

        // No repo: unavailable, and it says which of the three reasons.
        let (_home, _vault, app) = app_with_vault(None);
        let out = call(&app, "backup_latest", serde_json::json!({ "vault": "notes" })).unwrap();
        assert!(out.contains("\"vault\":\"notes\""), "{out}");
        assert!(out.contains("\"id\":null"), "{out}");
        if backup::available() {
            assert!(out.contains("no media-backup repository"), "{out}");
        } else {
            // On a machine with no restic that reason is reported first, and correctly so:
            // there is no point naming a missing repo to someone who has no tool to use it.
            assert!(out.contains("restic is not installed"), "{out}");
        }

        // A repo but no password: a different sentence again.
        let (_home2, _vault2, app) = app_with_vault(Some("/tmp/no-such-restic-repo"));
        let out = call(&app, "backup_latest", serde_json::json!({ "vault": "notes" })).unwrap();
        assert!(out.contains("\"unavailable\":"), "{out}");
        if backup::available() {
            assert!(out.contains("no restic password"), "{out}");
        }
    }

    /// An unreachable repository is **reported, not raised**.
    ///
    /// A backup destination on another machine is unreachable as an ordinary matter, and a panel
    /// that throws on it tells the user less than one that names the repo it could not open. This
    /// is the assertion that keeps `backup_latest` returning `Ok` on a restic failure.
    #[test]
    fn backup_latest_reports_an_unreadable_repo_rather_than_failing() {
        let _guard = ENV.lock().unwrap_or_else(|e| e.into_inner());
        if !backup::available() {
            eprintln!("skipping: restic is not installed, so there is no failure path to drive");
            return;
        }
        let (_home, _vault, app) = app_with_vault(Some("/tmp/definitely-not-a-restic-repo"));
        crate::secrets::save_restic_password("wrong password").unwrap();

        let out = call(&app, "backup_latest", serde_json::json!({ "vault": "notes" }))
            .expect("an unreachable repo is a reported state, never an error");
        assert!(out.contains("could not read"), "{out}");
        assert!(out.contains("definitely-not-a-restic-repo"), "must name it: {out}");
        crate::secrets::clear_restic_password().unwrap();
    }

    /// **A panic must not brick the process.**
    ///
    /// Before 2026-09-04 `lock()` propagated the `PoisonError`, so the *next* command — and every
    /// command after it, from every client, for the life of the process — answered with the
    /// `Display` of a poison error. The agent thread and the webview both dispatch, so a panic in
    /// either took out both. `paper.rs` records this happening for real: one multi-byte character
    /// in one PDF, and the app was unusable until restart.
    ///
    /// Deterministic — no timing, no threads of its own, no subprocess.
    #[test]
    fn a_panic_while_holding_the_vault_lock_does_not_brick_every_later_command() {
        let _guard = ENV.lock().unwrap_or_else(|e| e.into_inner());
        let (_home, _vault, app) = app_with_vault(None);

        // Prove the app works, poison it, prove it still works.
        assert!(call(&app, "list_vaults", serde_json::json!({})).is_ok());
        app.poison_for_test();
        let out = call(&app, "list_vaults", serde_json::json!({}))
            .expect("a poisoned lock must be recovered, not propagated");
        assert!(out.contains("notes"), "and it must still answer correctly: {out}");

        // A *write* has to survive it too — recovery that only served reads would move the failure
        // rather than remove it.
        call(&app, "capture", serde_json::json!({ "body": "after the panic", "vault": "notes" }))
            .expect("writing must work after a recovered poisoning");
    }
    /// **Point restic's cache at a directory this test owns, and unset it again on the way out.**
    ///
    /// `RESTIC_CACHE_DIR` is process-global and `backup::backup` has no parameter for it. Every
    /// test in this module already takes `ENV`, so nothing races *while* it is set — but the
    /// variable would outlive the `TempDir` it names, and the next test to run restic
    /// (`backup_latest_reports_an_unreadable_repo_rather_than_failing`) would inherit a cache
    /// directory deleted underneath it: `mkdir …/data/67: no such file or directory`, which reads
    /// like a permissions problem and is not. That is the trap `known-issues.md` records; in
    /// `fm-core` it took out three tests that never touched the variable at all.
    ///
    /// A `Drop` rather than a line at the end of the test, so a panic part-way cannot leave it
    /// set. The struct's `Drop` runs before its field's, so the variable is gone before the
    /// directory is.
    struct ResticCache(#[allow(dead_code)] tempfile::TempDir);

    impl ResticCache {
        fn scoped() -> Self {
            let dir = tempfile::tempdir().unwrap();
            std::env::set_var("RESTIC_CACHE_DIR", dir.path());
            Self(dir)
        }
    }

    impl Drop for ResticCache {
        fn drop(&mut self) {
            std::env::remove_var("RESTIC_CACHE_DIR");
        }
    }

    /// **The third way a vault comes into being, and the one with no dispatch test** — the last
    /// residue of `outstanding.md` §2.10.
    ///
    /// `restore_vault`'s own doc comment promises the shape: *"validate everything cheap and pure
    /// first, so a mistyped field refuses while the disk is untouched"*. Nothing asserted it, so
    /// what is asserted here is not the wording but the **absence of the destination**: a refusal
    /// that has already created what it declined to fill has half-done the thing it said no to,
    /// and the corrected second attempt then meets a directory that is in its way.
    #[test]
    fn restore_vault_refuses_before_it_creates_the_destination() {
        let _guard = ENV.lock().unwrap_or_else(|e| e.into_inner());
        let (_home, _vault, app) = app_with_vault(None);
        let parent = tempfile::tempdir().unwrap();
        let dest = parent.path().join("recovered");

        // A name already in use — the cheapest check, and the first one.
        let e = call(
            &app,
            "restore_vault",
            serde_json::json!({
                "name": "notes",
                "path": dest.to_str().unwrap(),
                "repo": "/tmp/no-such-restic-repo",
            }),
        )
        .unwrap_err();
        assert!(e.contains("already a vault called 'notes'"), "{e}");
        assert!(!dest.exists(), "a refusal must not have created the destination");

        // A good name and nothing to restore from. The refusal has to name the missing field:
        // an empty repo is otherwise indistinguishable from a restore that failed.
        let e = call(
            &app,
            "restore_vault",
            serde_json::json!({
                "name": "recovered", "path": dest.to_str().unwrap(), "repo": "  ",
            }),
        )
        .unwrap_err();
        assert!(e.contains("restic repository"), "{e}");
        assert!(!dest.exists(), "still nothing on disk");
    }

    /// **The round trip through the dispatch arm, and the one thing only this layer does.**
    ///
    /// `fm-core` covers the restore itself, including that a custom notes directory survives —
    /// but its test writes the descriptor back *inside the test*. The dispatch arm is the only
    /// production code that writes it, and until now nothing executed that line. Without it the
    /// restore succeeds perfectly and the vault opens looking in `notes/`: every note on disk,
    /// every view empty, and nothing on screen to say why.
    ///
    /// So the assertions are the three that belong here rather than one level down — the notes
    /// are visible **through the app**, the vault is in `vaults.json`, and the repo it came from
    /// is remembered there. That last one matters more than it reads: a restored vault has no
    /// remote and no history, so the restic repo is the only thing connecting it to anywhere.
    ///
    /// Proven red by deleting the `if restored.notes_dir != "notes"` block — the restore still
    /// reports success, `vaults.json` is still correct, and `recent` comes back empty.
    #[test]
    fn restore_vault_makes_the_notes_visible_and_remembers_the_repo() {
        let _guard = ENV.lock().unwrap_or_else(|e| e.into_inner());
        if !backup::available() {
            eprintln!("skipping: restic is not installed, so there is nothing to restore from");
            return;
        }
        let _cache = ResticCache::scoped();
        let password = "correct horse battery staple";

        // A source vault that keeps its notes in `docs/` — the case this arm exists for.
        let source = tempfile::tempdir().unwrap();
        std::fs::write(source.path().join("vault.json"), r#"{"notes":"docs"}"#).unwrap();
        {
            let mut s = fm_core::FileStore::named(source.path(), "source").unwrap();
            let mut note = fm_model::Object::new(fm_model::Kind::Note, "kept in docs");
            note.title = Some("the recovered note".into());
            s.put(&note).unwrap();
        }
        let repo = tempfile::tempdir().unwrap();
        backup::backup(source.path(), repo.path(), password).unwrap();

        let (home, _vault, app) = app_with_vault(None);
        crate::secrets::save_restic_password(password).unwrap();
        let parent = tempfile::tempdir().unwrap();
        let dest = parent.path().join("recovered");

        let out = call(
            &app,
            "restore_vault",
            serde_json::json!({
                "name": "recovered",
                "path": dest.to_str().unwrap(),
                "repo": repo.path().to_str().unwrap(),
            }),
        )
        .expect("the restore must succeed");
        assert!(out.contains("recovered"), "the new vault is in the answer: {out}");

        // The descriptor written back — this arm's own contribution, and nothing else's.
        let desc = std::fs::read_to_string(dest.join("vault.json")).expect("a descriptor");
        assert!(desc.contains("docs"), "the notes directory must be recorded: {desc}");

        // And the note is visible *through the app*, not merely present on disk.
        let seen = call(&app, "recent", serde_json::json!({})).unwrap();
        assert!(seen.contains("the recovered note"), "the app must see it: {seen}");

        // Registered, and it remembers where it came from.
        let list = std::fs::read_to_string(home.path().join("vaults.json")).unwrap();
        assert!(list.contains("recovered"), "{list}");
        assert!(
            list.contains(repo.path().to_str().unwrap()),
            "the restic repo it was restored from must be remembered: {list}"
        );

        crate::secrets::clear_restic_password().unwrap();
    }
}
