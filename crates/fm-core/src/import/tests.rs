//! What an import must get right.
//!
//! The parsers and [`rewrite`] are pure, so most of this needs no filesystem at all — which is
//! the point of the split. The handful of tests that do touch disk are the ones asserting things
//! only disk can show: that the **source folder is never written to**, that a traversal out of it
//! is refused, and that running an import twice does not duplicate a vault.

use super::*;
use std::collections::HashMap;
use tempfile::tempdir;

// ── Logseq: the outline ─────────────────────────────────────────────────────────

#[test]
fn an_outline_keeps_its_nesting_as_markdown_lists() {
    let page = logseq::parse("- one\n\t- two\n\t\t- three\n- back\n", "Notes");
    assert_eq!(page.body, "- one\n  - two\n    - three\n- back");
}

#[test]
fn two_space_indentation_nests_the_same_as_tabs() {
    let tabs = logseq::parse("- a\n\t- b\n", "P").body;
    let spaces = logseq::parse("- a\n  - b\n", "P").body;
    assert_eq!(tabs, spaces);
}

/// The standing ruling on inline actions: a checkbox "stays plain Markdown in the body … and does
/// not auto-appear in any planning view". So a marker becomes a checkbox and **nothing is lifted
/// to `status`** — that is the behaviour, not a shortfall.
#[test]
fn task_markers_become_checkboxes_and_never_a_status() {
    let page = logseq::parse(
        "- TODO write it\n- DOING still writing\n- LATER someday\n- DONE finished\n- CANCELED dropped\n",
        "Tasks",
    );
    assert_eq!(
        page.body,
        "- [ ] write it\n- [ ] still writing\n- [ ] someday\n- [x] finished\n- [x] dropped"
    );
    assert_eq!(page.status, None, "a block marker must not become the note's status");
}

/// The bug a naive `starts_with` would ship: an ordinary sentence beginning with the same letters.
#[test]
fn a_marker_is_only_a_marker_as_a_whole_word() {
    let page = logseq::parse("- DOINGS of the committee\n- Later that evening\n", "P");
    assert_eq!(page.body, "- DOINGS of the committee\n- Later that evening");
}

#[test]
fn a_priority_cookie_is_dropped_but_its_text_survives() {
    assert_eq!(logseq::parse("- TODO [#A] urgent thing\n", "P").body, "- [ ] urgent thing");
}

#[test]
fn a_logbook_span_is_dropped_whole() {
    let page = logseq::parse(
        "- DONE a task\n  :LOGBOOK:\n  CLOCK: [2026-09-01 Mon 10:00:00]--[2026-09-01 Mon 10:30:00]\n  :END:\n- next\n",
        "P",
    );
    assert_eq!(page.body, "- [x] a task\n- next");
}

// ── Logseq: properties ──────────────────────────────────────────────────────────

#[test]
fn page_properties_become_note_fields_and_the_rest_become_custom_ones() {
    let page = logseq::parse(
        "title:: Real Title\ntags:: alpha, beta\nstatus:: doing\ndeadline:: <2026-09-02 Wed>\nyear:: 2017\n\n- body\n",
        "From Filename",
    );
    assert_eq!(page.title, "Real Title");
    assert_eq!(page.tags, vec!["alpha".to_string(), "beta".to_string()]);
    assert_eq!(page.status.as_deref(), Some("doing"));
    assert_eq!(page.due.map(|d| d.to_string()).as_deref(), Some("2026-09-02"));
    assert_eq!(page.props.get("year"), Some(&PropertyValue::Int(2017)));
    assert_eq!(page.body, "- body");
}

/// `decisions.md`, "A tag may contain a space, and both doors must agree what that means" — the
/// entry written *because* splitting on spaces breaks exactly this: an imported keyword.
#[test]
fn a_tag_containing_a_space_survives_as_one_tag() {
    let page = logseq::parse("tags:: Machine Learning, Notes\n", "P");
    assert_eq!(page.tags, vec!["Machine Learning".to_string(), "Notes".to_string()]);
}

#[test]
fn a_block_property_stays_as_text_but_bookkeeping_does_not() {
    let page = logseq::parse("- a block\n  source:: a book\n  id:: 66d3a1f2-0000-4000-8000-000000000001\n  collapsed:: true\n", "P");
    assert_eq!(page.body, "- a block\n  source: a book");
    assert_eq!(
        page.block_texts.get("66d3a1f2-0000-4000-8000-000000000001").map(String::as_str),
        Some("a block"),
        "an id:: must be captured before it is dropped — it is what a ((ref)) points at"
    );
}

#[test]
fn prose_containing_a_double_colon_is_not_a_property() {
    let page = logseq::parse("- see Foo::Bar in the code\n", "P");
    assert_eq!(page.body, "- see Foo::Bar in the code");
}

// ── tags and links, as tokens ───────────────────────────────────────────────────

#[test]
fn hashtags_are_hoisted_including_the_bracketed_multiword_form() {
    let page = logseq::parse("- a #project and #[[deep work]] here\n", "P");
    assert!(page.tags.contains(&"project".to_string()));
    assert!(page.tags.contains(&"deep work".to_string()));
}

#[test]
fn a_url_fragment_is_not_a_tag() {
    let mut tags = Vec::new();
    scan_hashtags("see https://example.com/page#section for more", &mut tags);
    assert!(tags.is_empty(), "got {tags:?}");
}

#[test]
fn a_markdown_heading_is_not_a_tag() {
    let mut tags = Vec::new();
    scan_hashtags("# Heading", &mut tags);
    assert!(tags.is_empty(), "got {tags:?}");
}

/// A bracketed hashtag shares Logseq's link syntax but is not a link. Counting it as one
/// reported every multi-word tag as a dangling link — and would have rewritten it into
/// `#[label](note:…)` the moment some page happened to share its name.
#[test]
fn a_bracketed_hashtag_is_a_tag_and_never_a_link() {
    let page = logseq::parse("- tagged #[[deep learning]] here\n", "P");
    assert!(page.tags.contains(&"deep learning".to_string()));
    assert!(page.links.is_empty(), "a tag was counted as a link: {:?}", page.links);

    // And even when a page by that name exists, the tag passes through untouched.
    let id = new_id();
    let g = graph_with(&[("deep learning", id)]);
    let mut s = Rewritten::default();
    assert_eq!(rewrite("tagged #[[deep learning]]", &g, &mut s), "tagged #[[deep learning]]");
    assert_eq!(s.links_resolved, 0);
    assert!(s.dangling.is_empty());
}

// ── the rewrite ─────────────────────────────────────────────────────────────────

fn graph_with(pages: &[(&str, Id)]) -> Graph {
    let mut g = Graph::default();
    for (name, id) in pages {
        g.add_page(name, *id);
    }
    g
}

#[test]
fn a_wikilink_becomes_a_note_reference() {
    let id = new_id();
    let g = graph_with(&[("Target Page", id)]);
    let mut s = Rewritten::default();
    let out = rewrite("see [[Target Page]] now", &g, &mut s);
    assert_eq!(out, format!("see [Target Page](note:{id}) now"));
    assert_eq!(s.links_resolved, 1);
}

#[test]
fn an_alias_is_what_the_reader_sees_and_a_heading_anchor_is_dropped() {
    let id = new_id();
    let g = graph_with(&[("Target", id)]);
    let mut s = Rewritten::default();
    assert_eq!(rewrite("[[Target|shown]]", &g, &mut s), format!("[shown](note:{id})"));
    assert_eq!(rewrite("[[Target#Section]]", &g, &mut s), format!("[Target](note:{id})"));
}

#[test]
fn a_link_resolves_regardless_of_case() {
    let id = new_id();
    let g = graph_with(&[("Target Page", id)]);
    let mut s = Rewritten::default();
    assert_eq!(rewrite("[[target page]]", &g, &mut s), format!("[target page](note:{id})"));
}

/// A page that exists only as a reference is normal in Logseq. Left verbatim and **counted** —
/// inventing a note for each would put an empty card in every board, agenda, timeline and feed.
#[test]
fn a_dangling_link_is_left_alone_and_counted() {
    let g = Graph::default();
    let mut s = Rewritten::default();
    assert_eq!(rewrite("[[Nowhere]]", &g, &mut s), "[[Nowhere]]");
    assert_eq!(s.dangling, vec!["Nowhere".to_string()]);
    assert_eq!(s.links_resolved, 0);
}

#[test]
fn a_block_reference_is_replaced_by_what_that_block_says() {
    let mut g = Graph::default();
    g.blocks.insert("abc12345".into(), "the quoted words".into());
    let mut s = Rewritten::default();
    assert_eq!(rewrite("as in ((abc12345))", &g, &mut s), "as in the quoted words");
    assert_eq!(s.blocks_inlined, 1);
}

#[test]
fn a_block_reference_we_did_not_import_says_so_instead_of_leaving_debris() {
    let g = Graph::default();
    let mut s = Rewritten::default();
    let out = rewrite("as in ((66d3a1f2-0000-4000-8000-000000000001))", &g, &mut s);
    assert!(out.contains("not imported"), "{out}");
    assert!(!out.contains("66d3a1f2"));
    assert_eq!(s.blocks_unresolved, 1);
}

#[test]
fn an_embed_becomes_the_embed_spelling_of_a_note_reference() {
    let id = new_id();
    let g = graph_with(&[("Some Page", id)]);
    let mut s = Rewritten::default();
    assert_eq!(rewrite("{{embed [[Some Page]]}}", &g, &mut s), format!("![Some Page](note:{id})"));
    assert_eq!(rewrite("![[Some Page]]", &g, &mut s), format!("![Some Page](note:{id})"));
}

#[test]
fn an_attachment_becomes_a_blob_reference_in_both_spellings() {
    let hash = "a".repeat(64);
    let mut g = Graph::default();
    g.add_asset("pic.png", Asset { hash: hash.clone(), filename: "pic.png".into() });
    g.add_asset("../assets/pic.png", Asset { hash: hash.clone(), filename: "pic.png".into() });
    let mut s = Rewritten::default();
    assert_eq!(rewrite("![[pic.png]]", &g, &mut s), format!("![pic.png](asset:sha256-{hash})"));
    assert_eq!(
        rewrite("![a shot](../assets/pic.png)", &g, &mut s),
        format!("![a shot](asset:sha256-{hash})")
    );
}

#[test]
fn somebody_elses_url_is_left_exactly_as_it_is() {
    let g = Graph::default();
    let mut s = Rewritten::default();
    let body = "![remote](https://example.com/x.png) and [docs](https://example.com)";
    assert_eq!(rewrite(body, &g, &mut s), body);
}

#[test]
fn a_label_that_could_close_its_own_bracket_is_escaped() {
    let id = new_id();
    let g = graph_with(&[("a]b", id)]);
    let mut s = Rewritten::default();
    assert_eq!(rewrite("[[a]b]]", &g, &mut s), format!("[a\\]b](note:{id})"));
}

#[test]
fn unicode_prose_survives_the_rewrite_byte_for_byte() {
    let g = Graph::default();
    let mut s = Rewritten::default();
    let body = "café → π ≈ 3.14 — naïve";
    assert_eq!(rewrite(body, &g, &mut s), body);
}

// ── Obsidian ────────────────────────────────────────────────────────────────────

#[test]
fn obsidian_frontmatter_maps_onto_note_fields() {
    let page = obsidian::parse(
        "---\ntags: [alpha, beta]\naliases:\n  - Other Name\nstatus: doing\ncreated: 2021-03-04\nrating: 5\n---\nthe body\n",
        "Note",
    );
    assert_eq!(page.tags, vec!["alpha".to_string(), "beta".to_string()]);
    assert_eq!(page.aliases, vec!["Other Name".to_string()]);
    assert_eq!(page.status.as_deref(), Some("doing"));
    assert_eq!(page.created.map(|d| d.date().to_string()).as_deref(), Some("2021-03-04"));
    assert_eq!(page.props.get("rating"), Some(&PropertyValue::Int(5)));
    assert_eq!(page.body, "the body");
}

#[test]
fn an_obsidian_note_without_frontmatter_is_not_an_error() {
    let page = obsidian::parse("just text with a [[Link]]\n", "Note");
    assert_eq!(page.body, "just text with a [[Link]]");
    assert_eq!(page.links, vec!["Link".to_string()]);
    assert_eq!(page.title, "Note");
}

#[test]
fn frontmatter_that_is_not_yaml_is_reported_not_swallowed() {
    let page = obsidian::parse("---\n: : :\n\tbad\n---\nbody\n", "Note");
    assert_eq!(page.body, "body");
    assert!(!page.warnings.is_empty(), "a broken frontmatter block must be reported");
}

/// Obsidian writes a link to a note in a subfolder either way — `[[Roadmap]]` or
/// `[[Projects/Roadmap]]` — and both resolve there. Registering only the filename dangled every
/// folder-qualified link in the vault, which in a foldered vault is most of them.
#[test]
fn an_obsidian_link_resolves_by_folder_path_as_well_as_by_name() {
    let src = tempdir().unwrap();
    let vault = tempdir().unwrap();
    std::fs::create_dir_all(src.path().join(".obsidian")).unwrap();
    std::fs::write(src.path().join(".obsidian/app.json"), "{}").unwrap();
    std::fs::create_dir_all(src.path().join("Projects")).unwrap();
    std::fs::write(src.path().join("Projects/Roadmap.md"), "the plan\n").unwrap();
    std::fs::write(src.path().join("Notes.md"), "see [[Projects/Roadmap|the plan]]\n").unwrap();

    let out = convert(src.path(), vault.path(), &HashMap::new(), Options::default()).unwrap();
    let roadmap = out.notes.iter().find(|n| n.title.as_deref() == Some("Roadmap")).unwrap();
    let notes = out.notes.iter().find(|n| n.title.as_deref() == Some("Notes")).unwrap();
    assert_eq!(notes.body, format!("see [the plan](note:{})", roadmap.id));
    assert_eq!(out.report.dangling, 0);
}

// ── dates ───────────────────────────────────────────────────────────────────────

#[test]
fn the_date_spellings_both_apps_use_all_parse() {
    for raw in ["<2026-09-02 Wed>", "[[2026-09-02]]", "2026-09-02"] {
        assert_eq!(parse_date_value(raw).map(|d| d.to_string()).as_deref(), Some("2026-09-02"), "{raw}");
    }
    assert_eq!(
        parse_date_value("<2026-09-02 Wed 14:30>").map(|d| d.to_string()).as_deref(),
        Some("2026-09-02T14:30")
    );
    assert_eq!(parse_date_value("someday"), None);
}

#[test]
fn a_journal_filename_is_the_day_it_stands_for() {
    assert_eq!(journal_date("2026_09_02").map(|d| d.date().to_string()).as_deref(), Some("2026-09-02"));
    assert_eq!(journal_date("not a date"), None);
}

/// Logseq escapes a namespace separator in the filename, and has used two spellings for it. Get
/// this wrong and every link to a namespaced page silently dangles.
#[test]
fn a_namespaced_filename_unescapes_to_its_page_name() {
    assert_eq!(page_name("Parent___Child"), "Parent/Child");
    assert_eq!(page_name("Parent%2FChild"), "Parent/Child");
}

// ── the whole thing, on disk ────────────────────────────────────────────────────

/// A small but representative Logseq graph.
fn logseq_graph(root: &Path) {
    let w = |p: &str, s: &[u8]| {
        let path = root.join(p);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, s).unwrap();
    };
    w("logseq/config.edn", b"{}");
    w(
        "pages/Project Alpha.md",
        b"tags:: work, Machine Learning\n- goal is [[Deep Work]]\n- TODO ship it\n  id:: block-one-2345\n- ![shot](../assets/pic.png)\n",
    );
    w("pages/Deep Work.md", b"- see ((block-one-2345)) and [[Nowhere At All]]\n");
    w("journals/2026_09_02.md", b"- wrote things\n");
    w("assets/pic.png", b"\x89PNG\r\n\x1a\nfake");
}

#[test]
fn a_logseq_graph_becomes_notes_with_resolved_links_and_a_stored_attachment() {
    let src = tempdir().unwrap();
    let vault = tempdir().unwrap();
    logseq_graph(src.path());

    let out = convert(src.path(), vault.path(), &HashMap::new(), Options::default()).unwrap();

    assert_eq!(out.format, Format::Logseq);
    assert_eq!(out.report.notes_added, 3, "two pages and a journal");
    assert_eq!(out.attachments.len(), 1, "the png is stored once");

    let alpha = out.notes.iter().find(|n| n.title.as_deref() == Some("Project Alpha")).unwrap();
    assert!(alpha.tags.contains(&"Machine Learning".to_string()), "{:?}", alpha.tags);
    let deep = out.notes.iter().find(|n| n.title.as_deref() == Some("Deep Work")).unwrap();
    assert!(alpha.body.contains(&format!("note:{}", deep.id)), "link not resolved: {}", alpha.body);

    // The block reference resolved to the words that block says, across two files.
    assert!(deep.body.contains("ship it"), "block ref not inlined: {}", deep.body);
    // And a page nobody defined stayed put rather than becoming an empty note.
    assert!(deep.body.contains("[[Nowhere At All]]"));
    assert_eq!(out.report.stubs_created, 0);

    // The attachment is a blob reference, and the blob is really in the store.
    let hash = &out.attachments[0].hash;
    assert!(alpha.body.contains(&format!("asset:sha256-{hash}")), "{}", alpha.body);
    assert!(crate::BlobStore::new(vault.path()).exists(hash));

    // A journal carries the day it names, not the day the import ran.
    let journal = out.notes.iter().find(|n| n.tags.contains(&"journal".to_string())).unwrap();
    assert_eq!(journal.created.date().to_string(), "2026-09-02");
    // Titled by the day it stands for. `2026_09_02` is a filename, not something to read.
    assert_eq!(journal.title.as_deref(), Some("2026-09-02"));

    // Every note says where it came from, so a second import can recognise it.
    assert_eq!(
        alpha.extra.get(SOURCE_LIBRARY),
        Some(&PropertyValue::Text("logseq".into()))
    );
    assert!(matches!(alpha.extra.get(SOURCE_KEY), Some(PropertyValue::Text(k)) if k.ends_with("Project Alpha.md")));
}

/// **The invariant the whole design rests on.** V4 exists to avoid writing into folders the user
/// owns elsewhere; an importer that edited the graph would be exactly that mistake.
#[test]
fn the_source_folder_is_never_written_to() {
    let src = tempdir().unwrap();
    let vault = tempdir().unwrap();
    logseq_graph(src.path());

    let before = fingerprint(src.path());
    convert(src.path(), vault.path(), &HashMap::new(), Options::default()).unwrap();
    assert_eq!(before, fingerprint(src.path()), "the graph was modified by importing it");
}

#[test]
fn importing_twice_adds_nothing_the_second_time() {
    let src = tempdir().unwrap();
    let vault = tempdir().unwrap();
    logseq_graph(src.path());

    let first = convert(src.path(), vault.path(), &HashMap::new(), Options::default()).unwrap();
    let existing: HashMap<String, Id> = first
        .notes
        .iter()
        .filter_map(|n| match n.extra.get(SOURCE_KEY) {
            Some(PropertyValue::Text(k)) => Some((k.clone(), n.id)),
            _ => None,
        })
        .collect();

    let second = convert(src.path(), vault.path(), &existing, Options::default()).unwrap();
    assert_eq!(second.report.notes_added, 0, "a re-import must not duplicate the vault");
    assert_eq!(second.report.notes_already_imported, 3);
    assert!(second.notes.is_empty());
}

/// A source property must not take a name the note format already owns.
///
/// `to_file` writes `extra` into the **same** YAML mapping as the well-known keys, and
/// `Mapping::insert` overwrites — so an imported `type::` or `updated::` would replace ours, and
/// `from_file` would then fail to parse the note at all, making it vanish from every view as an
/// unreadable file. `base::` is worse and quieter: `thread::notes_base` *hides* a note that has
/// one, so it would import successfully and simply never appear.
#[test]
fn a_source_property_cannot_take_a_name_the_note_format_owns() {
    let src = tempdir().unwrap();
    let vault = tempdir().unwrap();
    std::fs::create_dir_all(src.path().join("logseq")).unwrap();
    std::fs::write(src.path().join("logseq/config.edn"), "{}").unwrap();
    std::fs::create_dir_all(src.path().join("pages")).unwrap();
    std::fs::write(
        src.path().join("pages/P.md"),
        "type:: paper\nupdated:: nonsense\nbase:: something\ncreated:: 2017-06-12\n\n- body\n",
    )
    .unwrap();

    let out = convert(src.path(), vault.path(), &HashMap::new(), Options::default()).unwrap();
    let note = &out.notes[0];

    for taken in ["logseq_type", "logseq_updated", "logseq_base"] {
        assert!(note.extra.contains_key(taken), "{taken} missing from {:?}", note.extra);
    }
    assert_eq!(out.report.renamed_properties, 3);

    // A page that states its own creation date is believed — that is better than the mtime, and
    // it is why `created` is consumed rather than renamed.
    assert_eq!(note.created.date().to_string(), "2017-06-12");

    // The real proof: it round-trips through the file format it will be written in. Without the
    // guard this is where an imported note stops being readable.
    let text = crate::frontmatter::to_file(note).unwrap();
    let back = crate::frontmatter::from_file(&text).expect("an imported note must be readable");
    assert_eq!(back.id, note.id);
    assert_eq!(back.created, note.created);
}

/// A body is untrusted text — it arrives from whoever wrote the graph.
#[test]
fn an_attachment_path_cannot_climb_out_of_the_source_folder() {
    let src = tempdir().unwrap();
    let vault = tempdir().unwrap();
    let secret = src.path().parent().unwrap().join("outside-secret.txt");
    std::fs::write(&secret, "private").unwrap();

    std::fs::create_dir_all(src.path().join("logseq")).unwrap();
    std::fs::write(src.path().join("logseq/config.edn"), "{}").unwrap();
    std::fs::create_dir_all(src.path().join("pages")).unwrap();
    std::fs::write(
        src.path().join("pages/Evil.md"),
        "- ![x](../outside-secret.txt)\n",
    )
    .unwrap();

    let out = convert(src.path(), vault.path(), &HashMap::new(), Options::default()).unwrap();
    assert_eq!(out.attachments.len(), 0, "a file outside the graph must never be ingested");
    let _ = std::fs::remove_file(&secret);
}

#[test]
fn a_vault_inside_the_graph_is_refused_before_anything_is_written() {
    let src = tempdir().unwrap();
    logseq_graph(src.path());
    let inside = src.path().join("my-vault");
    std::fs::create_dir_all(&inside).unwrap();

    let err = match convert(src.path(), &inside, &HashMap::new(), Options::default()) {
        Err(e) => e,
        Ok(_) => panic!("a vault inside the graph was accepted"),
    };
    assert!(err.to_string().contains("inside one another"), "{err}");
}

#[test]
fn stubs_are_created_only_when_asked_for() {
    let src = tempdir().unwrap();
    let vault = tempdir().unwrap();
    logseq_graph(src.path());

    let out = convert(src.path(), vault.path(), &HashMap::new(), Options { create_stubs: true }).unwrap();
    assert!(out.report.stubs_created > 0);
    let stub = out.notes.iter().find(|n| n.title.as_deref() == Some("Nowhere At All")).unwrap();
    assert!(stub.tags.contains(&"imported-stub".to_string()));
}

#[test]
fn a_folder_that_is_neither_is_refused_with_a_reason() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("notes.md"), "hello").unwrap();
    let s = scan(dir.path());
    assert!(!s.ok());
    assert!(s.problems[0].contains("Logseq") && s.problems[0].contains("Obsidian"), "{:?}", s.problems);
}

#[test]
fn the_preview_counts_what_is_there_without_parsing_it() {
    let src = tempdir().unwrap();
    logseq_graph(src.path());
    let s = scan(src.path());
    assert!(s.ok());
    assert_eq!(s.format, Some(Format::Logseq));
    assert_eq!(s.journals, 1);
    assert_eq!(s.pages, 2);
    assert_eq!(s.attachments, 1);
}

/// Every file under `root` and its bytes — so a test can prove nothing moved.
fn fingerprint(root: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files = Vec::new();
    walk(root, &mut files).unwrap();
    let mut out: Vec<(String, Vec<u8>)> = files
        .iter()
        .map(|p| {
            (
                p.strip_prefix(root).unwrap().to_string_lossy().into_owned(),
                std::fs::read(p).unwrap_or_default(),
            )
        })
        .collect();
    out.sort();
    out
}
