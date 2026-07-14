//! `fm` — the CLI that drives formicarium's core without the GUI. Enough to
//! exercise each slice end-to-end: capture notes to disk, rebuild the index from
//! the files, search, and edit properties. The desktop app (fm-app) reuses the
//! same core.

use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand};
use fm_core::{FileStore, Reindex, Store};
use fm_model::{Id, Kind, Object, PropertyValue};
use fm_query::{Filter, Predicate, Query, SortKey};
use std::path::PathBuf;
use std::str::FromStr;
use time::macros::format_description;
use time::{Date, OffsetDateTime};

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
                bail!("nothing to capture");
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
            apply_property(&mut obj, &key, &value.join(" "))?;
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
    }
    Ok(())
}

/// Parse a string value into the right typed field, the write-side mirror of
/// `Object::get`: well-known keys become typed fields, anything else a custom
/// `extra` property. An empty value clears an optional or custom property.
fn apply_property(obj: &mut Object, key: &str, raw: &str) -> Result<()> {
    let raw = raw.trim();
    let some = |s: &str| (!s.is_empty()).then(|| s.to_string());
    match key {
        "status" => obj.status = some(raw),
        "title" => obj.title = some(raw),
        "type" | "kind" => obj.kind = Kind::from_str(raw).map_err(|e| anyhow!(e))?,
        "hard" => obj.hard = matches!(raw, "true" | "yes" | "1"),
        "due" => {
            obj.due = match some(raw) {
                Some(s) => Some(
                    Date::parse(&s, &format_description!("[year]-[month]-[day]"))
                        .context("due must be YYYY-MM-DD")?,
                ),
                None => None,
            }
        }
        "tags" => {
            obj.tags = raw
                .split([',', ' '])
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(String::from)
                .collect()
        }
        "id" | "created" | "updated" | "schema" => bail!("`{key}` is not editable"),
        other => {
            if raw.is_empty() {
                obj.extra.remove(other);
            } else {
                obj.extra.insert(other.to_string(), PropertyValue::Text(raw.to_string()));
            }
        }
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
