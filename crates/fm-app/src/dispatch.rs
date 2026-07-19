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
use crate::vaults::{self, VaultConfig};
use fm_core::{backup, git, vcs, MultiStore, Reindex, Store};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

/// The open vaults and the list describing them, **behind one lock**.
///
/// One and not two, for a reason that is already latent in [`dispatch`]: its arms take these
/// in *opposite orders* — `commit`/`push` lock the store and then resolve the vault, while
/// `ingest` resolves the vault and then locks the store. Two mutexes would make that a
/// textbook AB/BA deadlock. One deletes the question, and closes the window where the list
/// holds a vault the store doesn't.
pub struct Vaults {
    store: MultiStore,
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
/// entire command — and five arms exist precisely to *drop* it before doing slow I/O
/// (`asset_status`, `resolve_asset`, `open_external`, `backup`, `backup_status`).
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
        let vaults::VaultList { vaults, path: config, writable: config_writable } =
            vaults::load();
        let store = MultiStore::open(
            &vaults.iter().map(|v| (v.name.clone(), v.path.clone())).collect::<Vec<_>>(),
        )
        .map_err(|e| format!("open vaults: {e}"))?;
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
        App { vaults: Mutex::new(Vaults { store, list }), config, config_writable }
    }

    /// An owned snapshot of the configured vaults — what a shell prints at startup.
    pub fn configs(&self) -> Result<Vec<VaultConfig>, String> {
        Ok(self.lock()?.configs())
    }

    fn lock(&self) -> Result<MutexGuard<'_, Vaults>, String> {
        self.vaults.lock().map_err(|e| e.to_string())
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
pub fn dispatch(
    cmd: &str,
    args: &Value,
    body: &[u8],
    app: &App,
    host: &dyn Host,
) -> Result<Output, String> {
    let s = |k: &str| args.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let lock = || app.lock();

    match cmd {
        "board" => json(commands::board(&lock()?.store, &s("groupBy")).map_err(err)?),
        "gallery" => json(commands::gallery(&lock()?.store).map_err(err)?),
        "agenda" => json(commands::agenda(&lock()?.store).map_err(err)?),
        "get" => json(commands::get(&lock()?.store, &s("id")).map_err(err)?),
        "search" => json(commands::search(&lock()?.store, &s("query")).map_err(err)?),
        "recent" => json(commands::recent(&lock()?.store).map_err(err)?),
        // The collaboration read-model: who last edited each note, and when, straight from each
        // vault's git log — one command behind the authorship labels, the activity stream, and
        // the contributor filter. Aggregated across vaults, newest-first.
        "activity" => {
            let since = {
                let s = s("since");
                if s.is_empty() { "1 year ago".to_string() } else { s }
            };
            let g = lock()?;
            let mut all = Vec::new();
            for cfg in g.configs() {
                all.extend(commands::activity(&g.store, &cfg.path, &since).map_err(err)?);
            }
            all.sort_by(|a, b| b.time.cmp(&a.time));
            json(all)
        }
        "capture" => {
            // Validate the target vault up front (unknown name → a loud error, never a
            // silent default), then route the note into it — the create-side twin of
            // `ingest`. Empty picks the default vault.
            let mut g = lock()?;
            let into = g.config(&s("vault"))?;
            json(commands::capture(&mut g.store, &s("body"), &into.name).map_err(err)?)
        }
        "set_property" => {
            commands::set_property(&mut lock()?.store, &s("id"), &s("key"), &s("value"))
                .map_err(err)?;
            nothing()
        }
        // Answers with the new `updated` stamp, which the caller holds and sends back as
        // `base` on its next write — that round trip is the lost-update guard for an editor
        // that has been open across someone else's pull. See `commands::update_body`.
        "update_body" => {
            let stamp =
                commands::update_body(&mut lock()?.store, &s("id"), &s("body"), &s("base"))
                    .map_err(err)?;
            json(stamp)
        }
        "delete" => {
            commands::delete(&mut lock()?.store, &s("id")).map_err(err)?;
            nothing()
        }
        // Searched across vaults: the reference names bytes, not a place. Configs are
        // cloned out and the guard dropped before touching the disk.
        "asset_status" => {
            let r = s("reference");
            let (vaults, default) = {
                let g = lock()?;
                (g.configs(), g.config("").ok())
            };
            let found = vaults
                .iter()
                .find_map(|v| commands::asset_status(&v.path, &r).ok().filter(|st| st.has_blob));
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
            let path = lock()?.find_blob(&s("reference"))?;
            host.open_external(&path)?;
            nothing()
        }
        "commit" => {
            // Hold the lock so a commit can't snapshot the vault mid-write (a server is
            // thread-per-connection). One guard, where this used to take two: resolving
            // the vault through the same guard is what keeps that from being a
            // self-deadlock now that the store and the list share a lock.
            let mut g = lock()?;
            let cfg = g.config(&s("vault"))?;
            // Exactly the files this app wrote or deleted — not a directory, and certainly
            // not `-A`. A vault may also be a repo you commit to yourself, and this fires
            // five seconds after every save.
            let paths = g.store.written(&cfg.name);
            let made = vcs::commit_all(&cfg.path, &s("message"), &paths).map_err(err)?;
            // Cleared only once the commit actually landed: a failed commit that forgot its
            // list would leave those notes unstaged forever.
            if made {
                g.store.clear_written(&cfg.name);
            }
            json(made)
        }
        "backup" => {
            run_backup(app, &s("vault"))?;
            nothing()
        }
        // What the two backup tiers would actually do right now — the panel needs
        // this to promise the user only what it can deliver.
        "backup_status" => json(backup_status(app)?),
        // The identity rides along because this is the one moment it is worth
        // asking for: a vault gaining a remote is a vault gaining an audience, and
        // from here on every commit carries a name into somebody else's clone.
        // Empty means "don't touch it" — a user whose git is already configured is
        // never asked, so the panel sends nothing.
        "set_git_remote" => {
            let (name, email) = (s("name"), s("email"));
            // Resolved once, where it used to be resolved twice.
            let path = lock()?.config(&s("vault"))?.path;
            if !name.is_empty() || !email.is_empty() {
                vcs::set_identity(&path, &name, &email).map_err(err)?;
            }
            vcs::set_remote(&path, &s("url")).map_err(err)?;
            nothing()
        }
        "push" => {
            // Same reason as `commit`: don't let a push snapshot the vault mid-write.
            let g = lock()?;
            let path = g.config(&s("vault"))?.path;
            json(vcs::push_squashed(&path, &s("message")).map_err(err)?)
        }
        // Bring a collaborator's work home. Holds the lock for the same reason push does
        // — a merge rewrites notes under the app's feet, and the very next incremental
        // reindex is what makes them visible.
        "pull" => {
            let mut g = lock()?;
            let path = g.config(&s("vault"))?.path;
            let outcome = vcs::pull(&path).map_err(err)?;
            // The merge just wrote files behind the index's back. Re-read now rather
            // than leave the user staring at pre-pull content until the next heartbeat.
            g.store.reindex(Reindex::Incremental).map_err(err)?;
            json(match outcome {
                git::Pulled::UpToDate => PullResult { merged: 0, conflicts: Vec::new() },
                git::Pulled::Merged(n) => PullResult { merged: n, conflicts: Vec::new() },
                git::Pulled::Conflicted(f) => PullResult { merged: 0, conflicts: f },
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
        "ping" => {
            let mut g = lock()?;
            let changed = g.store.reindex(Reindex::Incremental).map_err(err)?;
            json(Ping {
                changed: changed.updated > 0 || changed.removed > 0,
                git: vcs::available(),
                restic: backup::available(),
                // Taken from the store rather than from `changed`, because this is the
                // *current* set across every vault, labelled by which one — not just what
                // this pass happened to re-read.
                skipped: g.store.skipped().iter().map(SkippedOut::from).collect(),
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
            let path = {
                let g = lock()?;
                g.store
                    .skipped()
                    .iter()
                    .find(|sk| sk.vault == vault && sk.name == name)
                    .map(|sk| sk.path.clone())
                    .ok_or_else(|| {
                        format!(
                            "not a currently-unreadable note: {vault}/{name} — it may have \
                             been fixed already"
                        )
                    })?
            };
            host.open_external(&path)?;
            nothing()
        }
        // The audiences that exist. `[]` is **the first-run signal** — the one command
        // that is meaningful with no vaults, and the reason it isn't folded into
        // `backup_status` (which shells out per vault, including a network `ls-remote`).
        "list_vaults" => {
            let g = lock()?;
            json(infos(&g.configs(), &g.store.names()))
        }
        // What would happen if we created a vault here — the form asks on every keystroke.
        "check_path" => {
            let g = lock()?;
            json(check_path(&g, app.config.as_deref(), app.config_writable, &s("name"), &s("path")))
        }
        "create_vault" => json(create_vault(app, &s("name"), &s("path"))?),
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
                vault_list: app.config.as_ref().map(|p| p.display().to_string()),
                vault_list_writable: app.config_writable,
                vaults: infos(&g.configs(), &g.store.names()),
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
                .filter_map(|k| std::env::var(k).ok().map(|v| EnvVar { name: (*k).into(), value: v }))
                .collect(),
                // Capabilities, not settings: things the machine either has or does not, which
                // change what the app can do and are the commonest source of "why is this
                // greyed out". `RESTIC_PASSWORD` is reported as present/absent only — never
                // its value, which is why it is a bool and not an `env` entry.
                git: fm_core::vcs::available(),
                restic_installed: backup::available(),
                restic_password_set: std::env::var("RESTIC_PASSWORD").is_ok_and(|v| !v.is_empty()),
            })
        }
        // The other way a vault comes into existence: someone else already has it. Same
        // registration as `create_vault`, with a clone in front and an identity behind.
        "clone_vault" => json(clone_vault(
            app,
            &s("name"),
            &s("path"),
            &s("url"),
            &s("gitName"),
            &s("gitEmail"),
        )?),
        // The third way in: a vault you already have, in a backup, on a machine that no
        // longer exists. Same registration as the other two, with a restic restore in front.
        "restore_vault" => json(restore_vault(app, &s("name"), &s("path"), &s("repo"))?),
        // The user's saved `.view` files, aggregated across every vault: a view is
        // git-tracked *in* the vault it belongs to, but the query it defines runs against
        // the whole set (so `prop: vault` can narrow, or a dashboard can span audiences).
        // A view that won't parse is listed with its error, never dropped.
        "list_views" => {
            let g = lock()?;
            let mut all = Vec::new();
            for v in g.configs() {
                all.extend(crate::views::list_views(&v.path));
            }
            json(all)
        }
        // Run one view by name. The file is read from whichever vault holds it; the query
        // runs against the full store. A parse error surfaces as the error body, so a broken
        // view says why rather than silently returning nothing.
        "run_view" => {
            let name = s("name");
            let g = lock()?;
            let vault_path = g
                .configs()
                .iter()
                .find(|v| crate::views::list_views(&v.path).iter().any(|vi| vi.name == name))
                .map(|v| v.path.clone())
                .ok_or_else(|| format!("no view named '{name}'"))?;
            json(crate::views::run_view(&g.store, &vault_path, &name).map_err(err)?)
        }
        // Binary upload: the raw `body` IS the file, which is exactly why its name and
        // vault arrive as `args` rather than in it.
        "ingest" => {
            let name = {
                let n = s("name");
                if n.is_empty() { "asset".to_string() } else { n }
            };
            // Into the vault the caller names — the blob lands beside the notes that
            // will reference it, and never in an audience that shouldn't have it. Both
            // the path and the name, so the bytes and the asset note land in the *same*
            // vault: splitting them puts the file in one audience and its note in another.
            //
            // The owned `config` is what makes this borrow-check: `&mut g.store` and a
            // `&VaultConfig` borrowed from the same guard cannot coexist.
            let mut g = lock()?;
            let into = g.config(&s("vault"))?;
            json(commands::ingest(&mut g.store, &into.path, &into.name, &name, body).map_err(err)?)
        }
        // Copy a note into another vault. Restrictive by default (only the prose travels);
        // `with_assets` opts in to carrying the first-degree blobs. Validate the target up
        // front, hand `copy_note` every vault's (name, path) so it can locate/copy blobs, then
        // version the new note in its own vault (best-effort — a solo user's own repo).
        "copy_note" => {
            let with_assets = args.get("with_assets").and_then(Value::as_bool).unwrap_or(false);
            let mut g = lock()?;
            let into = g.config(&s("vault"))?;
            let vault_paths: Vec<(String, PathBuf)> =
                g.configs().into_iter().map(|c| (c.name, c.path)).collect();
            let result =
                commands::copy_note(&mut g.store, &s("id"), &into.name, &vault_paths, with_assets)
                    .map_err(err)?;
            if vcs::available() {
                let paths = g.store.written(&into.name);
                if vcs::commit_all(&into.path, "backup: copy note", &paths).unwrap_or(false) {
                    g.store.clear_written(&into.name);
                }
            }
            json(result)
        }
        // Pre-check for the copy popover: does the target vault already hold a copy of this
        // note? Drives the "this will replace the existing copy" warning.
        "copy_status" => {
            let g = lock()?;
            let into = g.config(&s("vault"))?;
            json(commands::copy_status(&g.store, &s("id"), &into.name).map_err(err)?)
        }
        // Recede a copy: delete the copied note from the target vault and take back the blobs
        // this copy newly wrote (only those nothing else there still references).
        "uncopy_note" => {
            let blobs: Vec<String> = args
                .get("blobs")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let mut g = lock()?;
            let into = g.config(&s("vault"))?;
            let vault_paths: Vec<(String, PathBuf)> =
                g.configs().into_iter().map(|c| (c.name, c.path)).collect();
            commands::uncopy_note(&mut g.store, &s("id"), &into.name, &blobs, &vault_paths)
                .map_err(err)?;
            if vcs::available() {
                let paths = g.store.written(&into.name);
                if vcs::commit_all(&into.path, "backup: undo copy", &paths).unwrap_or(false) {
                    g.store.clear_written(&into.name);
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
pub fn blob_path(app: &App, reference: &str) -> Result<PathBuf, String> {
    app.lock()?.find_blob(reference)
}

impl Vaults {
    /// A named vault; an empty name means the default (the first).
    ///
    /// Returns an **owned** config, not a reference: a `&VaultConfig` borrowed from the
    /// guard would conflict with `&mut self.store` in the very arms that need both
    /// (`ingest`), and it is three allocations against a caller that is usually about to
    /// fork `git`.
    ///
    /// An **unknown** name is an error, never a fallback. Quietly writing a note meant
    /// for "lab" into "personal" is a disclosure that git history makes permanent, and
    /// the reverse silently loses the note — so a typo has to be loud. With **no** vaults
    /// there is no default to fall back to either, which is the first run and says so.
    fn config(&self, name: &str) -> Result<VaultConfig, String> {
        if self.list.is_empty() {
            return Err("no vaults configured — create one first".into());
        }
        if name.is_empty() {
            return Ok(self.list[0].clone());
        }
        self.list
            .iter()
            .find(|v| v.name == name)
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
        self.store.add(store);
        self.list.push(cfg);
    }

    /// Where a blob really is. **Every vault is searched**, because a `sha256:`
    /// reference deliberately does not say which vault holds the bytes — and it should
    /// not: that is what keeps a cross-vault `note:`/`asset:` link free, and what lets
    /// ULIDs be the only identifier anyone needs. Content-addressing makes searching
    /// *correct* rather than merely convenient: whichever vault answers, the bytes hash
    /// to the reference, so they are the same bytes. Vault-scoping references would
    /// re-couple a note to a location and break the links.
    fn find_blob(&self, reference: &str) -> Result<PathBuf, String> {
        for v in &self.list {
            if let Ok(p) = commands::blob_path(&v.path, reference) {
                return Ok(p);
            }
        }
        Err(format!("blob not present in any vault: {reference}"))
    }
}

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

fn json<T: serde::Serialize>(v: T) -> Result<Output, String> {
    serde_json::to_vec(&v).map(Output::Json).map_err(|e| e.to_string())
}

/// Success with nothing to say. An empty JSON payload, not `null` — the commands that use
/// this report by not failing.
fn nothing() -> Result<Output, String> {
    Ok(Output::Json(Vec::new()))
}

/// The heartbeat's answer: did anything change on disk that the tab is not showing?
/// A bool, not a count — the UI's only choice is whether to re-run its query.
#[derive(serde::Serialize)]
struct Ping {
    changed: bool,
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
        Self { vault: s.vault.clone(), name: s.name.clone(), reason: s.reason.clone() }
    }
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
    restic_password_set: bool,
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
/// secret appears here — `RESTIC_PASSWORD` is reported as a bool and never by value.
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
    /// this vault has a repo, and `RESTIC_PASSWORD` is set. All three, because "ready"
    /// must mean "will work" — gating on configuration alone offers a checkbox that ticks
    /// and then fails on a machine with no restic.
    restic_ready: bool,
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
}

fn backup_status(app: &App) -> Result<BackupStatus, String> {
    // One password for every repo. A per-vault password would have to live somewhere,
    // and the one place it must never live is the config file next to the paths.
    let has_password = std::env::var("RESTIC_PASSWORD").is_ok();
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
        })
        .collect();
    Ok(BackupStatus { vaults, git: vcs::available(), restic: has_restic })
}

/// The media tier, for **one** vault — snapshot it into *its own* restic repo.
///
/// Per vault because a restic repo is per repository: there is no single destination
/// that could hold a set of vaults, so each one either has a repo of its own or has
/// nowhere for its media to go. A vault without one is not an error and must not be
/// silently folded into someone else's repo — the caller is told, by name, that this
/// vault's media stayed put. Refusing to say so is the overstatement this whole panel
/// exists to prevent.
fn run_backup(app: &App, vault: &str) -> Result<(), String> {
    // Owned, and the guard dropped: restic can take minutes, and nothing else may write
    // notes meanwhile — but everything else may read them.
    let v = app.lock()?.config(vault)?;
    let repo = v.restic.as_ref().ok_or_else(|| {
        format!("no restic repo configured for '{}' — its media has nowhere to go", v.name)
    })?;
    let password = std::env::var("RESTIC_PASSWORD")
        .map_err(|_| "set RESTIC_PASSWORD for the restic repository".to_string())?;
    backup::backup(&v.path, Path::new(repo), &password)
        .map_err(|e| format!("backing up '{}': {e}", v.name))
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

    PathCheck { facts, name_ok, name_taken, path_taken, overlaps, config_writable, ok }
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
    let cfg = VaultConfig { name: name.to_string(), path: path.clone(), restic: None };
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
    let names = g.store.names();
    Ok(infos(&g.configs(), &names))
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
        return Err("a shared vault needs your name and email — they sign every commit you \
                    make in it, and your collaborators see them"
            .into());
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

    let cfg = VaultConfig { name: name.to_string(), path: path.clone(), restic: None };
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
    let names = g.store.names();
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
        return Err("restic is not installed on this machine, so there is no backup to \
                    restore from — a vault can still be created here, or cloned with git"
            .into());
    }
    let password = std::env::var("RESTIC_PASSWORD")
        .ok()
        .filter(|p| !p.is_empty())
        .ok_or("set RESTIC_PASSWORD to the repository's password — the app never stores it")?;
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
    let names = g.store.names();
    Ok(infos(&g.configs(), &names))
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
    v.iter()
        .enumerate()
        .map(|(i, e)| VaultInfo {
            name: if e.name.is_empty() {
                store_names.get(i).map(|n| n.to_string()).unwrap_or_default()
            } else {
                e.name.clone()
            },
            path: e.path.to_string_lossy().into_owned(),
            default: i == 0,
        })
        .collect()
}
