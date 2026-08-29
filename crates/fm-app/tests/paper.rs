//! The offline half of turning a citation into a paper note: what can be recognised without a
//! network, which is all of it, because the core links no HTTP client by the owner's ruling.

use fm_app::paper::{
    identifier_in_text, parse_bibtex, parse_identifier, to_bibtex, Identifier, PaperFields,
};
use std::collections::BTreeMap;

#[test]
fn a_doi_is_recognised_however_it_was_pasted() {
    let bare = Identifier::Doi("10.48550/arXiv.1706.03762".into());
    for input in [
        "10.48550/arXiv.1706.03762",
        "  10.48550/arXiv.1706.03762  ",
        "doi:10.48550/arXiv.1706.03762",
        "https://doi.org/10.48550/arXiv.1706.03762",
        "See (doi:10.48550/arXiv.1706.03762).",
    ] {
        assert_eq!(parse_identifier(input), Some(bare.clone()), "input: {input:?}");
    }
    // Normalised to the bare form, so one paper pasted two ways is one value a `Prop{Eq}` matches.
    assert_eq!(bare.key(), "doi");
}

#[test]
fn an_arxiv_id_is_recognised_but_never_guessed() {
    assert_eq!(parse_identifier("arXiv:2401.12345"), Some(Identifier::ArXiv("2401.12345".into())));
    assert_eq!(parse_identifier("arXiv:2401.12345v2"), Some(Identifier::ArXiv("2401.12345v2".into())));
    assert_eq!(
        parse_identifier("https://arxiv.org/abs/1706.03762"),
        Some(Identifier::ArXiv("1706.03762".into()))
    );
    assert_eq!(
        parse_identifier("https://arxiv.org/pdf/1706.03762.pdf"),
        Some(Identifier::ArXiv("1706.03762".into()))
    );
    // **A bare `2401.12345` is not an identifier.** In running text it is as likely a figure
    // number or a price, and guessing wrong writes a false citation onto somebody's note.
    assert_eq!(parse_identifier("2401.12345"), None);
    assert_eq!(parse_identifier("see table 2401.12345 for detail"), None);
}

#[test]
fn a_url_is_recorded_rather_than_guessed_at() {
    assert_eq!(
        parse_identifier("https://example.org/some/paper"),
        Some(Identifier::Url("https://example.org/some/paper".into()))
    );
    assert_eq!(parse_identifier("Attention Is All You Need"), None, "a title is not an identifier");
    assert_eq!(parse_identifier(""), None);
}

#[test]
fn the_identifier_a_pdf_prints_on_itself_comes_from_the_front_only() {
    let front = "Attention Is All You Need\nAshish Vaswani\narXiv:1706.03762v7 [cs.CL]\n\nAbstract…";
    assert_eq!(identifier_in_text(front), Some(Identifier::ArXiv("1706.03762v7".into())));

    // **A DOI in the bibliography belongs to somebody else's paper.** Only the front matter is
    // scanned, so a reference list cannot rename the note it is attached to.
    let mut body = String::from("A paper with no identifier of its own.\n");
    body.push_str(&"filler ".repeat(1200));
    body.push_str("\n[14] Someone. doi:10.1000/notmine\n");
    assert_eq!(identifier_in_text(&body), None, "a cited DOI is not this paper's DOI");
}

#[test]
fn a_pasted_bibtex_entry_becomes_the_fields() {
    let entry = r#"
@inproceedings{vaswani2017attention,
  title     = {Attention Is All You Need},
  author    = {Vaswani, Ashish and Shazeer, Noam and Parmar, Niki},
  booktitle = {Advances in Neural Information Processing Systems},
  year      = {2017},
  doi       = {10.48550/arXiv.1706.03762}
}
"#;
    let got = parse_bibtex(entry).expect("parses");
    assert_eq!(got.title.as_deref(), Some("Attention Is All You Need"));
    // `and` becomes `; ` — a "Last, First" name contains a comma, so it must not be the separator.
    assert_eq!(got.authors.as_deref(), Some("Vaswani, Ashish; Shazeer, Noam; Parmar, Niki"));
    assert_eq!(got.year.as_deref(), Some("2017"));
    assert_eq!(got.venue.as_deref(), Some("Advances in Neural Information Processing Systems"));
    assert_eq!(got.doi.as_deref(), Some("10.48550/arXiv.1706.03762"));
    assert_eq!(got.entry_type.as_deref(), Some("inproceedings"));
    assert_eq!(got.cite_key.as_deref(), Some("vaswani2017attention"));
}

#[test]
fn bibtex_survives_the_shapes_exporters_actually_emit() {
    // Quoted values, a comma *inside* a braced value, brace-protected capitalisation, and a
    // trailing comma — all of which appear in real exports.
    let entry = r#"@article{k, title = "{BERT}: Pre-training of Deep Models",
      author={Devlin, J. and Chang, M.},
      journal={Proc. of X, Y and Z}, year="2019",}"#;
    let got = parse_bibtex(entry).expect("parses");
    assert_eq!(got.title.as_deref(), Some("BERT: Pre-training of Deep Models"));
    assert_eq!(got.venue.as_deref(), Some("Proc. of X, Y and Z"), "a comma inside braces is not a separator");
    assert_eq!(got.year.as_deref(), Some("2019"));

    // biblatex's `date` narrows to the year, because that is what a paper note groups by.
    let d = parse_bibtex("@book{k, title={T}, date={2021-06-12}}").expect("parses");
    assert_eq!(d.year.as_deref(), Some("2021"));

    // Not BibTeX at all → None, so the caller can fall through to identifier or title.
    assert!(parse_bibtex("Attention Is All You Need").is_none());
    assert!(parse_bibtex("").is_none());
    assert!(parse_bibtex("@").is_none());
}

#[test]
fn a_paper_note_can_be_copied_back_out_as_bibtex() {
    let mut props = BTreeMap::new();
    props.insert("authors".to_string(), "Vaswani, Ashish; Shazeer, Noam".to_string());
    props.insert("year".to_string(), "2017".to_string());
    props.insert("venue".to_string(), "NeurIPS".to_string());
    props.insert("doi".to_string(), "10.48550/arXiv.1706.03762".to_string());
    props.insert("entry_type".to_string(), "inproceedings".to_string());

    let out = to_bibtex("Attention Is All You Need", &props);
    assert!(out.starts_with("@inproceedings{vaswani2017,"), "key falls back to author+year:\n{out}");
    assert!(out.contains("title = {Attention Is All You Need}"), "{out}");
    // Back to BibTeX's own separator — the exact inverse of the parse.
    assert!(out.contains("author = {Vaswani, Ashish and Shazeer, Noam}"), "{out}");
    assert!(out.contains("doi = {10.48550/arXiv.1706.03762}"), "{out}");
    assert!(out.trim_end().ends_with('}'), "{out}");

    // And it round-trips: what we emit, we can read back.
    let back = parse_bibtex(&out).expect("our own output parses");
    assert_eq!(back.title.as_deref(), Some("Attention Is All You Need"));
    assert_eq!(back.authors.as_deref(), Some("Vaswani, Ashish; Shazeer, Noam"));
    assert_eq!(back.year.as_deref(), Some("2017"));
}

#[test]
fn the_venue_is_emitted_under_the_field_its_entry_type_takes() {
    // A round trip through our own parser cannot catch this — `booktitle` and `journal` both map
    // back to `venue` — but an `@inproceedings` carrying `journal =` is wrong in LaTeX.
    let mut props = BTreeMap::new();
    props.insert("venue".to_string(), "NeurIPS".to_string());
    props.insert("entry_type".to_string(), "inproceedings".to_string());
    assert!(to_bibtex("T", &props).contains("booktitle = {NeurIPS}"), "{:?}", to_bibtex("T", &props));

    props.insert("entry_type".to_string(), "article".to_string());
    assert!(to_bibtex("T", &props).contains("journal = {NeurIPS}"));

    props.insert("entry_type".to_string(), "phdthesis".to_string());
    assert!(to_bibtex("T", &props).contains("school = {NeurIPS}"));

    props.insert("entry_type".to_string(), "book".to_string());
    assert!(to_bibtex("T", &props).contains("publisher = {NeurIPS}"));
}

#[test]
fn the_fields_become_frontmatter_in_a_fixed_order() {
    let f = PaperFields {
        title: Some("T".into()),
        authors: Some("A".into()),
        year: Some("2017".into()),
        doi: Some("10.1/x".into()),
        ..Default::default()
    };
    // `title` is a well-known field, not a custom property, so it is not in the list.
    let keys: Vec<_> = f.properties().into_iter().map(|(k, _)| k).collect();
    assert_eq!(keys, vec!["authors", "year", "doi"], "fixed order, empties dropped");
    assert!(PaperFields::default().is_empty());
}
