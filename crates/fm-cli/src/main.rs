//! `fm` — the CLI that drives formicarium's core without the GUI. Enough to
//! prove S0 end-to-end: capture a note to disk, rebuild the index from the
//! files, and read it back. The desktop app (fm-app) reuses the same core.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use fm_core::{FileStore, Reindex, Store};
use fm_model::{Kind, Object};
use fm_query::{Filter, Predicate, Query, SortKey};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "fm", about = "formicarium — local research notebook")]
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
    /// List all notes, newest first.
    List,
    /// Full-text search across all notes.
    Search { query: Vec<String> },
    /// Rebuild the index from the files on disk.
    Reindex,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    // Opening the vault rebuilds the index from files — this IS "reindex on
    // reload": every invocation reads the current on-disk truth.
    let mut store = FileStore::open(&cli.vault)
        .with_context(|| format!("opening vault at {}", cli.vault.display()))?;

    match cli.cmd {
        Cmd::Capture { text } => {
            let body = text.join(" ");
            if body.trim().is_empty() {
                anyhow::bail!("nothing to capture");
            }
            let obj = Object::new(Kind::Note, body);
            let id = obj.id;
            store.put(&obj)?;
            println!("captured {id}  ->  {}/notes/{id}.md", cli.vault.display());
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
                sort: vec![SortKey::desc("created")],
                ..Default::default()
            };
            let r = store.query(&q)?;
            println!("{} match(es) for \"{needle}\":", r.total);
            for o in &r.rows {
                println!("  {}  {}", o.id, first_line(&o.body));
            }
        }
        Cmd::Reindex => {
            let stats = store.reindex(Reindex::Full)?;
            println!("reindexed: {} file(s) scanned, {} indexed", stats.scanned, stats.updated);
        }
    }
    Ok(())
}

fn first_line(body: &str) -> &str {
    body.lines().next().unwrap_or("").trim()
}
