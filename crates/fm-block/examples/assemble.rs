//! **uBlock Origin's resources for the phone's pages** (`privacy-plan-2026-11-19.md` 3.2, 3.3), as
//! Brave's packager assembles them: the stand-ins from `src/web_accessible_resources` (indexed by
//! `src/js/redirect-resources.js`), plus the scriptlets (`ci/ubo-scriptlets.mjs` turns uBO's ES module
//! into `Resource`s). Run by `pixi run block-resources`, never by the app:
//! `cargo run -p fm-block --features assemble --example assemble -- <uBlock dir> <scriptlets.json> <out.json>`.
use std::path::Path;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let [dir, scriptlets, out] = &a[..] else {
        eprintln!("usage: assemble <uBlock dir> <scriptlets.json> <out.json>");
        std::process::exit(2);
    };
    let dir = Path::new(dir);
    let mut all = adblock::resources::resource_assembler::assemble_web_accessible_resources(
        &dir.join("src/web_accessible_resources"),
        &dir.join("src/js/redirect-resources.js"),
    );
    // Brave leaves this one out (it needs Google's own player); so does formicaria.
    all.retain(|r| r.name != "google-ima-dai.js");
    let stand_ins = all.len();
    let s: Vec<adblock::resources::Resource> =
        serde_json::from_str(&std::fs::read_to_string(scriptlets).expect("scriptlets"))
            .expect("scriptlets json");
    let n = s.len();
    all.extend(s);
    // A check before it is kept: every resource loads into an engine.
    let mut e = fm_block::Blocker::new(&[]);
    let text = serde_json::to_string(&all).expect("json");
    assert_eq!(e.use_resources(&text), all.len(), "every resource reads back");
    std::fs::write(out, text).expect("write");
    println!("block-resources: {stand_ins} stand-ins, {n} scriptlets → {out}");
}
