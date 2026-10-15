//! **Does a Logseq graph survive the import?** Measured, not assumed.
//!
//! The plan, and why it is shaped like this: `docs/context/logseq-import-plan-2026-10-14.md`.
//!
//! - [`the_oracle_reads_the_mini_graph_as_logseq_shows_it`] — the measuring instrument is itself
//!   checked first, against a graph small enough to know by heart.
//! - [`the_mini_graph_conforms`] — the public fixture (`tests/fixtures/logseq-mini`), one page per
//!   construct a real graph was found to use. **Ratcheted**: the checks in [`KNOWN_FAILING`]
//!   are the importer's known gaps, and the test fails if one of them starts passing (so the list
//!   shrinks with every fix) as surely as it fails if any other check breaks.
//! - [`a_private_corpus_conforms`] — the owner's real graph, which never enters this repo.
//!   `FM_IMPORT_CORPUS=<graph> pixi run import-corpus`; the scorecard and the failures it names
//!   are written to `target/import-corpus/`, which git ignores.

mod import_conformance;

use import_conformance::{call, new_app, oracle, reopen_app};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn mini() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/logseq-mini")
}

/// The checks the importer fails today on the mini-graph. **Remove an id when its fix lands.**
const KNOWN_FAILING: &[&str] = &[];

#[test]
fn the_oracle_reads_the_mini_graph_as_logseq_shows_it() {
    let inv = oracle::inventory(&mini());
    assert_eq!(inv.pages.len(), 10, "8 pages and 2 journals");
    assert_eq!(inv.journal_format, "MMM do, yyyy", "a commented-out format is not the format");

    // Every name Logseq would answer to.
    for (name, file) in [
        ("Beta: Colon", "pages/Beta%3A Colon.md"),
        ("gamma title", "pages/gamma-file.md"),
        ("Gamma Alias", "pages/gamma-file.md"),
        ("Parent/Child", "pages/Parent___Child.md"),
        ("Oct 14th, 2026", "journals/2026_10_14.md"),
        ("@Paper: Zotero Example", "pages/@Paper%3A Zotero Example.md"),
    ] {
        assert_eq!(inv.page_named(name).map(|p| p.key.as_str()), Some(file), "{name}");
    }

    let page = |k: &str| inv.pages.iter().find(|p| p.key == k).unwrap();
    let alpha = page("pages/Alpha Page.md");
    assert_eq!(alpha.title, "Alpha Page");
    // Seven wikilinks, the `.md` link, and the embed; the two tags are not links.
    assert_eq!(alpha.links.len(), 7, "{:?}", alpha.links);
    assert_eq!(alpha.block_refs, 2, "a bare ((ref)) and a [label](((ref)))");
    assert!(alpha.words.contains("zebracactus") && !alpha.words.contains("todo"));

    let media = page("pages/Media.md");
    let present: Vec<_> = media.attachments.iter().filter(|a| a.file.is_some()).collect();
    assert_eq!(present.len(), 8, "{:#?}", media.attachments);
    assert_eq!(media.macros, ["video", "renderer", "query"]);
    assert!(!media.words.contains("youtube"), "a URL is not prose");

    let paper = page("pages/@Paper%3A Zotero Example.md");
    assert_eq!(paper.title, "@Paper: Zotero Example");
    let missing: Vec<_> = paper.attachments.iter().filter(|a| a.file.is_none()).collect();
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].basename(), "Example - 2020 - A missing paper.pdf");

    let hls = page("pages/hls__deck_1.md");
    assert_eq!(hls.highlights, [2]);
    assert!(hls.highlight_file.is_some());

    let code = page("pages/Math and Code.md");
    assert!(code.links.is_empty() && code.macros.is_empty(), "nothing inside a fence is markup");
    assert!(code.words.contains("okapis"));

    assert_eq!(page("journals/2026_10_14.md").title, "2026-10-14");
    assert_eq!(inv.drawings, ["draws/2026-10-14-10-00-00.excalidraw"]);
}

#[test]
fn the_mini_graph_conforms() {
    let work = tempfile::tempdir().unwrap();
    let card = import_conformance::run(&mini(), work.path());
    let failing = card.failing();
    let md = card.to_markdown();
    let fixed: Vec<_> = KNOWN_FAILING.iter().filter(|id| !failing.contains(id)).collect();
    let broken: Vec<_> = failing.iter().filter(|id| !KNOWN_FAILING.contains(id)).collect();
    assert!(broken.is_empty(), "checks that used to pass now fail: {broken:?}\n\n{md}");
    assert!(
        fixed.is_empty(),
        "checks now pass — take them out of KNOWN_FAILING: {fixed:?}\n\n{md}"
    );
}

#[test]
#[ignore = "needs FM_IMPORT_CORPUS=<a Logseq graph>; run with `pixi run import-corpus`"]
fn a_private_corpus_conforms() {
    let Some(source) = std::env::var_os("FM_IMPORT_CORPUS").map(PathBuf::from) else {
        eprintln!("FM_IMPORT_CORPUS is not set — nothing to measure");
        return;
    };
    // Under `target/`, never a tmpfs: a real graph's attachments are gigabytes, and `/tmp` on this
    // machine is memory.
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/import-corpus");
    let work = out.join("work");
    let _ = std::fs::remove_dir_all(&work);
    let card = import_conformance::run(&source, &work);
    card.write(&out);
    println!("{}", card.to_markdown());
    println!("written to {}", out.display());
    let failing = card.failing();
    assert!(failing.is_empty(), "failing: {failing:?} — see {}/report.md", out.display());
}

/// Copy the mini-graph somewhere writable — a test that edits or links needs a source it owns.
fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let (src, dst) = (e.path(), to.join(e.file_name()));
        if src.is_dir() {
            copy_tree(&src, &dst);
        } else {
            std::fs::copy(&src, &dst).unwrap();
        }
    }
}

fn import(app: &fm_app::App, source: &Path, vault: &str, path: &Path, extra: Value) -> Value {
    let mut args = json!({ "source": source.to_string_lossy(), "vault": vault,
                           "name": if vault.is_empty() { "imported" } else { "" },
                           "path": if vault.is_empty() { path.to_string_lossy().to_string() } else { String::new() } });
    for (k, v) in extra.as_object().unwrap() {
        args[k] = v.clone();
    }
    call(app, "run_import", args).unwrap()
}

fn note_by_source(vault: &Path, key: &str) -> fm_model::Object {
    use fm_core::Store;
    let store = fm_core::FileStore::open(vault).unwrap();
    store
        .candidates(&fm_query::Filter::new())
        .unwrap()
        .0
        .into_iter()
        .find(|o| {
            matches!(o.extra.get("source_key"), Some(fm_model::PropertyValue::Text(k)) if k == key)
        })
        .unwrap_or_else(|| panic!("no note from {key}"))
}

/// **A replace never writes over the person's work** (`decisions.md#data`, 2026-10-15): the note
/// they edited is kept exactly as they left it, and the rest is rewritten from the graph.
#[test]
fn replacing_an_import_keeps_what_the_person_edited() {
    let work = tempfile::tempdir().unwrap();
    let vault = work.path().join("vault");
    let app = new_app(work.path());
    import(&app, &mini(), "", &vault, json!({}));

    let edited = note_by_source(&vault, "pages/Alpha Page.md");
    let mine = format!("{}\n\nMy own paragraph, written in the app.", edited.body);
    call(&app, "update_body", json!({ "id": edited.id.to_string(), "body": mine, "base": "" }))
        .unwrap();

    let report = import(&app, &mini(), "imported", &vault, json!({ "replace": true }));
    assert_eq!(report["keptEdited"], 1, "{report}");
    assert_eq!(report["keptEditedNames"], json!(["Alpha Page"]), "{report}");
    assert_eq!(report["replaced"].as_u64().unwrap() + 1, 11, "every other note: {report}");
    drop(app);
    assert_eq!(note_by_source(&vault, "pages/Alpha Page.md").body, mine, "the edit survived");
}

/// A note from an import made before the `import_hash` existed is replaced only while its
/// `updated` is still the source file's modification time — which any save in the app moves.
#[test]
fn a_note_from_before_the_import_hash_is_replaced_only_if_unsaved() {
    use fm_core::Store;
    let work = tempfile::tempdir().unwrap();
    let vault = work.path().join("vault");
    let app = new_app(work.path());
    import(&app, &mini(), "", &vault, json!({}));
    drop(app);
    // Make every note look like the old importer's: no hash.
    {
        let mut store = fm_core::FileStore::open(&vault).unwrap();
        for mut o in store.candidates(&fm_query::Filter::new()).unwrap().0 {
            if o.extra.remove("import_hash").is_some() {
                store.put(&o).unwrap();
            }
        }
    }
    let app = reopen_app("imported", &vault);
    let beta = note_by_source(&vault, "pages/Beta%3A Colon.md");
    call(&app, "update_body", json!({ "id": beta.id.to_string(), "body": "changed", "base": "" }))
        .unwrap();
    let report = import(&app, &mini(), "imported", &vault, json!({ "replace": true }));
    assert_eq!(report["keptEdited"], 1, "{report}");
    assert_eq!(report["replaced"], 10, "{report}");
}

/// The person's rules: a kind left out and a size limit. What is left out is **named** in the
/// note that mentions it, with why — never silently dropped.
#[test]
fn attachment_rules_leave_out_what_was_asked_and_say_so() {
    let work = tempfile::tempdir().unwrap();
    let vault = work.path().join("vault");
    let app = new_app(work.path());
    // The PDFs are 30 bytes and the PNGs 69: a 50-byte limit takes the PDFs and leaves the PNGs.
    let report =
        import(&app, &mini(), "", &vault, json!({ "skipKinds": ["zip"], "maxMb": 0.00005 }));
    drop(app);
    assert!(report["leftOut"].as_u64().unwrap() >= 2, "{report}");
    let media = note_by_source(&vault, "pages/Media.md");
    assert!(
        media.body.contains("data.zip (data_1.zip, left out of the import: .zip files"),
        "{}",
        media.body
    );
    assert!(
        media.body.contains(
            "pic one (pic_1.png, left out of the import: 69 bytes, over the 50 bytes limit)"
        ),
        "{}",
        media.body
    );
    assert!(
        media.body.contains("[deck.pdf](asset:sha256-"),
        "a small file still came: {}",
        media.body
    );
    assert!(!media.body.contains("](../assets/"), "{}", media.body);
}

/// **A vault holds every byte it owns** (`decisions.md#vault`, 2026-10-15): an attachment is a copy,
/// never a link to the graph's file — so changing or deleting the graph cannot touch the vault.
#[cfg(unix)]
#[test]
fn an_imported_attachment_is_the_vaults_own_copy() {
    use std::os::unix::fs::MetadataExt;
    let work = tempfile::tempdir().unwrap();
    let graph = work.path().join("graph");
    copy_tree(&mini(), &graph);
    let vault = work.path().join("vault");
    let app = new_app(work.path());
    import(&app, &graph, "", &vault, json!({}));
    drop(app);
    let pic = graph.join("assets/pic_1.png");
    assert_eq!(pic.metadata().unwrap().nlink(), 1, "the graph's file is shared with nothing");
    let blob = fm_core::BlobStore::new(&vault).path_for(&fm_core::blob::sha256_file(&pic).unwrap());
    // The graph's file changes, then goes; the vault's copy is untouched.
    std::fs::write(&pic, b"changed in the old folder").unwrap();
    std::fs::remove_dir_all(&graph).unwrap();
    assert_eq!(std::fs::read(&blob).unwrap().len(), 69, "the vault kept its own bytes");
}

/// The preview names the kinds of attachment, largest first, so the person can choose.
#[test]
fn the_preview_lists_attachments_by_kind() {
    let work = tempfile::tempdir().unwrap();
    let app = new_app(work.path());
    let check: Value = call(
        &app,
        "check_import",
        json!({ "source": mini().to_string_lossy(), "vault": "", "name": "x",
                "path": work.path().join("x").to_string_lossy() }),
    )
    .unwrap();
    let kinds: Vec<&str> =
        check["kinds"].as_array().unwrap().iter().filter_map(|k| k["kind"].as_str()).collect();
    for k in ["png", "pdf", "zip", "svg"] {
        assert!(kinds.contains(&k), "{kinds:?}");
    }
    assert_eq!(check["drawings"], 1);
    assert_eq!(check["previous"], json!([]), "nothing imported yet");
}

/// **"Imported before" means imported into *this* vault.** With the same graph imported into a
/// second vault, an import into the first skipped every page, because the check looked across
/// every vault; a replace would have rewritten notes under the other vault's ids (2026-10-15).
#[test]
fn an_import_into_one_vault_does_not_see_another_vaults_copy() {
    let work = tempfile::tempdir().unwrap();
    let app = new_app(work.path());
    // One copy in a vault of its own…
    import(&app, &mini(), "", &work.path().join("first"), json!({}));
    // …and an empty second vault, imported into as an existing vault.
    call(
        &app,
        "create_vault",
        json!({ "name": "second", "path": work.path().join("second").to_string_lossy() }),
    )
    .unwrap();
    let report = import(&app, &mini(), "second", Path::new(""), json!({ "replace": true }));
    assert_eq!(report["alreadyImported"], 0, "{report}");
    assert_eq!(report["replaced"], 0, "{report}");
    assert_eq!(report["notes"], 11, "every page arrives in the second vault: {report}");
    drop(app);
    // And the first vault's notes are untouched by the second import.
    let first = fm_core::FileStore::open(work.path().join("first")).unwrap();
    use fm_core::Store;
    let n = first.candidates(&fm_query::Filter::new()).unwrap().0.len();
    assert!(n >= 11, "the first vault still has its notes ({n})");
}

/// **An import that writes no notes brings no files.** Re-running an add-only import over pages
/// already imported copied every attachment again and wrote a card per file (2026-10-15).
#[test]
fn an_import_that_adds_nothing_copies_nothing() {
    use fm_core::Store;
    let work = tempfile::tempdir().unwrap();
    let vault = work.path().join("vault");
    let app = new_app(work.path());
    import(&app, &mini(), "", &vault, json!({}));
    drop(app);
    let count = || {
        fm_core::FileStore::open(&vault)
            .unwrap()
            .candidates(&fm_query::Filter::new())
            .unwrap()
            .0
            .len()
    };
    let before = count();
    let app = reopen_app("imported", &vault);
    let report = import(&app, &mini(), "imported", &vault, json!({}));
    drop(app);
    assert_eq!(report["notes"], 0, "{report}");
    assert_eq!(report["attachments"], 0, "{report}");
    assert_eq!(report["deduped"], 0, "nothing was even looked at: {report}");
    assert_eq!(count(), before, "no new cards");
}
