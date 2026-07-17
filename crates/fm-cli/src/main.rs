//! `fm` — the CLI that drives formicaria's core without the GUI. Enough to
//! exercise each slice end-to-end: capture notes to disk, rebuild the index from
//! the files, search, and edit properties. The desktop app (fm-app) reuses the
//! same core.

use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand};
use fm_core::{merge, FileStore, Reindex, Store};
use fm_model::{Id, Kind, Object, PropertyValue};
use fm_query::{Filter, Predicate, Query, SortKey};
use std::path::PathBuf;
use std::str::FromStr;
use time::OffsetDateTime;

#[derive(Parser)]
#[command(name = "fm", about = "formicaria — local research notebook")]
struct Cli {
    /// Vault directory (notes + index). Defaults to ./vault or $FM_VAULT.
    #[arg(long, global = true, env = "FM_VAULT", default_value = "vault")]
    vault: PathBuf,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Capture a note; the text becomes the note body.
    Capture { text: Vec<String> },
    /// Ingest a file as an asset: hash it into the blob store (dedup), extract
    /// searchable text (pdftotext for PDFs), and create an asset note for it.
    Add { path: PathBuf },
    /// List all notes, newest first.
    List,
    /// Full-text search across all notes.
    Search { query: Vec<String> },
    /// Set a property on a note and write it back to disk (bumps `updated`).
    Set {
        /// The note id (ULID).
        id: String,
        /// Property: status, title, due, hard, tags, type, or any custom key.
        key: String,
        /// New value; empty clears an optional or custom property.
        value: Vec<String>,
    },
    /// Show one note's properties and body.
    Show { id: String },
    /// Rebuild the index from the files on disk.
    Reindex,
    /// Check vault integrity (report-only, like `git fsck`). `--scrub` re-hashes
    /// every blob to catch bit-rot. Exits non-zero if any error is found.
    Verify {
        #[arg(long)]
        scrub: bool,
    },
    /// Write manifest.json — the sha256 inventory of the blob store.
    Manifest,
    /// Back up the vault to a restic repo (created on first run). Password from
    /// --password or $RESTIC_PASSWORD.
    Backup {
        repo: PathBuf,
        #[arg(long, env = "RESTIC_PASSWORD", hide_env_values = true)]
        password: String,
    },
    /// Restore the latest snapshot into a directory — test your backups.
    Restore {
        repo: PathBuf,
        dest: PathBuf,
        #[arg(long, env = "RESTIC_PASSWORD", hide_env_values = true)]
        password: String,
    },
    /// Verify a restic repo. --read-data re-reads every pack (off-site scrub).
    Check {
        repo: PathBuf,
        #[arg(long)]
        read_data: bool,
        #[arg(long, env = "RESTIC_PASSWORD", hide_env_values = true)]
        password: String,
    },
    /// Git's merge driver for notes — git calls this, you don't. Merges frontmatter
    /// structurally (`updated` = the later, `tags` unite, `id`/`created` never move)
    /// so two people editing different paragraphs of one note don't conflict on the
    /// `updated:` line we rewrite on every save. Installed by `ensure_repo`; exits 1
    /// on a real conflict, as git's driver contract requires.
    #[command(hide = true)]
    MergeMd {
        /// %O — the common ancestor.
        base: PathBuf,
        /// %A — our version, and where git expects the answer written.
        ours: PathBuf,
        /// %B — their version.
        theirs: PathBuf,
        /// %L — conflict marker length.
        #[arg(default_value_t = 7)]
        marker_size: usize,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // The merge driver runs inside git's plumbing, on three temp files, with no vault
    // anywhere in sight — and `--vault` defaults to `./vault`, so opening one below
    // would have a `git pull` silently create a vault directory and an index as a side
    // effect. Answer before the store exists.
    if let Cmd::MergeMd { base, ours, theirs, marker_size } = &cli.cmd {
        return match merge::merge_files(base, ours, theirs, *marker_size)? {
            merge::Merged::Clean => Ok(()),
            // Not an error: git's contract is that a non-zero exit means "I left you a
            // conflict in %A", which is a merge outcome, not a failure. Saying so on
            // stderr would print noise on a perfectly normal pull.
            merge::Merged::Conflicted => std::process::exit(1),
        };
    }

    // Opening the vault rebuilds the index from files — this IS "reindex on
    // reload": every invocation reads the current on-disk truth.
    let mut store = FileStore::open(&cli.vault)
        .with_context(|| format!("opening vault at {}", cli.vault.display()))?;

    match cli.cmd {
        Cmd::Capture { text } => {
            let body = text.join(" ");
            if body.trim().is_empty() {
                bail!("nothing to capture");
            }
            let obj = Object::new(Kind::Note, body);
            let id = obj.id;
            store.put(&obj)?;
            println!("captured {id}  ->  {}/notes/{id}.md", cli.vault.display());
        }
        Cmd::Add { path } => {
            let ing = fm_core::ingest_file(&cli.vault, &path)
                .with_context(|| format!("ingesting {}", path.display()))?;
            // The asset note: filename as title, blob hash in `assets`, MIME as a
            // queryable property, and the extracted text as the body so it is
            // full-text searchable and travels with the notes (not the blob).
            let mut obj = Object::new(Kind::Asset, ing.text.clone().unwrap_or_default());
            obj.title = Some(ing.filename.clone());
            obj.assets = vec![format!("sha256:{}", ing.hash)];
            obj.extra.insert("mime".into(), PropertyValue::Text(ing.mime.clone()));
            let id = obj.id;
            store.put(&obj)?;
            // Fire-and-forget thumbnail; failure only degrades the gallery tile.
            let thumbed = fm_core::ingest::thumbnail(&cli.vault, &ing.hash).is_ok();
            let chars = ing.text.as_deref().map(str::len).unwrap_or(0);
            println!(
                "added asset {id}  ({}, sha256:{}…){}",
                ing.mime,
                &ing.hash[..12],
                if ing.deduped { "  [blob deduped]" } else { "" }
            );
            println!(
                "  {chars} searchable char(s){}",
                if thumbed { ", thumbnail generated" } else { "" }
            );
        }
        Cmd::List => {
            let q = Query { sort: vec![SortKey::desc("created")], ..Default::default() };
            let r = store.query(&q)?;
            println!("{} note(s):", r.total);
            for o in &r.rows {
                println!("  {}  {}  {}", o.id, o.created.date(), first_line(&o.body));
            }
        }
        Cmd::Search { query } => {
            let needle = query.join(" ");
            let q = Query {
                filter: Filter::new().and(Predicate::Text(needle.clone())),
                sort: vec![SortKey::desc("updated")], // results w/ timestamps: newest first
                ..Default::default()
            };
            let r = store.query(&q)?;
            println!("{} match(es) for \"{needle}\":", r.total);
            for o in &r.rows {
                println!("  {}  {}  {}", o.id, o.updated.date(), first_line(&o.body));
            }
        }
        Cmd::Set { id, key, value } => {
            let id = Id::from_str(&id).map_err(|_| anyhow!("invalid id: {id}"))?;
            let mut obj = store.get(id)?.ok_or_else(|| anyhow!("no note {id}"))?;
            // Shared with the app's `set_property` (board drag write-back), so a
            // dragged card and `fm set` change the file identically.
            fm_core::apply_property(&mut obj, &key, &value.join(" "))?;
            obj.updated = OffsetDateTime::now_utc();
            store.put(&obj)?;
            println!("set {key} on {id}");
        }
        Cmd::Show { id } => {
            let id = Id::from_str(&id).map_err(|_| anyhow!("invalid id: {id}"))?;
            let o = store.get(id)?.ok_or_else(|| anyhow!("no note {id}"))?;
            print_object(&o);
        }
        Cmd::Reindex => {
            let stats = store.reindex(Reindex::Full)?;
            println!("reindexed: {} file(s) scanned, {} indexed", stats.scanned, stats.updated);
        }
        Cmd::Verify { scrub } => {
            let report = fm_core::verify(&cli.vault, scrub)?;
            for issue in &report.issues {
                let tag = match issue.severity {
                    fm_core::Severity::Error => "ERROR",
                    fm_core::Severity::Warning => "warn ",
                };
                println!("  [{tag}] {}: {}", issue.target, issue.message);
            }
            println!(
                "verified {} note(s), {} blob(s){}: {} error(s), {} warning(s)",
                report.notes,
                report.blobs,
                if report.scrubbed { " (scrubbed)" } else { "" },
                report.errors(),
                report.warnings()
            );
            if !report.ok() {
                std::process::exit(1);
            }
        }
        Cmd::Manifest => {
            let manifest = fm_core::Manifest::build(&cli.vault)?;
            manifest.write(&cli.vault)?;
            println!("wrote manifest.json: {} blob(s) inventoried", manifest.blobs.len());
        }
        Cmd::Backup { repo, password } => {
            fm_core::backup::backup(&cli.vault, &repo, &password)?;
            println!("backed up {} -> restic repo {}", cli.vault.display(), repo.display());
        }
        Cmd::Restore { repo, dest, password } => {
            fm_core::backup::restore(&repo, &password, &dest)?;
            println!("restored latest snapshot -> {}", dest.display());
        }
        Cmd::Check { repo, read_data, password } => {
            fm_core::backup::check(&repo, &password, read_data)?;
            println!("restic repo OK{}", if read_data { " (data re-read)" } else { "" });
        }
        // Answered above, before the vault was opened.
        Cmd::MergeMd { .. } => unreachable!("handled before the store is opened"),
    }
    Ok(())
}

fn print_object(o: &Object) {
    println!("id      {}", o.id);
    println!("type    {}", o.kind.as_str());
    if let Some(t) = &o.title {
        println!("title   {t}");
    }
    if let Some(s) = &o.status {
        println!("status  {s}");
    }
    if let Some(d) = o.due {
        println!("due     {d}{}", if o.hard { "  (hard)" } else { "" });
    }
    if !o.tags.is_empty() {
        println!("tags    {}", o.tags.join(", "));
    }
    for (k, v) in &o.extra {
        println!("{k:<8}{}", v.display());
    }
    println!("created {}", o.created);
    println!("updated {}", o.updated);
    println!("\n{}", o.body);
}

fn first_line(body: &str) -> &str {
    body.lines().next().unwrap_or("").trim()
}
