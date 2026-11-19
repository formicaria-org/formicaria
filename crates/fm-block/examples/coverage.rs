//! **What each list set refuses** (`privacy-plan-2026-11-19.md` 1.4), offline: the corpus of real
//! requests (`tests/data/corpus.txt`) through the pages' engine, and the app corpus
//! (`tests/data/app-corpus.txt`) through the app blocker's names, for each set of lists given.
//! `cargo run --release -p fm-block --example coverage -- <lists dir>` (the lists as `<key>.txt`,
//! fetched by `pixi run privacy-coverage`). Prints Markdown tables for the baseline document.
use fm_block::hosts::HostSet;
use fm_block::Blocker;
use std::collections::BTreeMap;
use std::path::Path;

fn read(dir: &Path, keys: &[&str]) -> Vec<String> {
    keys.iter().filter_map(|k| std::fs::read_to_string(dir.join(format!("{k}.txt"))).ok()).collect()
}

fn main() {
    let dir = std::env::args().nth(1).expect("lists dir");
    let dir = Path::new(&dir);
    let corpus = include_str!("../tests/data/corpus.txt");
    let rows: Vec<(&str, &str, &str)> = corpus
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter_map(|l| {
            let mut p = l.split_whitespace();
            Some((p.next()?, p.next()?, p.next()?))
        })
        .collect();
    let sets: &[(&str, &[&str])] = &[
        ("formicaria 0.55 (EasyPrivacy)", &["easyprivacy"]),
        ("formicaria 0.55 (EasyPrivacy + EasyList)", &["easyprivacy", "easylist"]),
        (
            "now: trackers (+ uBO Privacy, Unbreak, Quick fixes, Badware)",
            &["easyprivacy", "ubo-privacy", "ubo-unbreak", "ubo-quick-fixes", "ubo-badware"],
        ),
        (
            "now: trackers and ads (+ uBO filters, Peter Lowe's)",
            &[
                "easyprivacy",
                "ubo-privacy",
                "ubo-unbreak",
                "ubo-quick-fixes",
                "ubo-badware",
                "easylist",
                "ubo-filters",
                "pgl",
            ],
        ),
    ];
    println!("| Lists | analytics | social | ads | ordinary refused |\n|---|---|---|---|---|");
    for (name, keys) in sets {
        let texts = read(dir, keys);
        let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
        let b = Blocker::new(&refs);
        let mut by: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
        let mut wrong = Vec::new();
        for (kind, ty, url) in &rows {
            let refused = b.check(url, "https://news.example/", ty);
            let e = by.entry(kind).or_default();
            e.1 += 1;
            if refused {
                e.0 += 1;
                if *kind == "ordinary" {
                    wrong.push(*url);
                }
            }
        }
        let cell = |k: &str| by.get(k).map_or("–".into(), |(r, n)| format!("{r} of {n}"));
        println!(
            "| {name} | {} | {} | {} | {}{} |",
            cell("analytics"),
            cell("social"),
            cell("ads"),
            cell("ordinary"),
            if wrong.is_empty() { String::new() } else { format!(" ({})", wrong.join(", ")) }
        );
    }
    let apps = include_str!("../tests/data/app-corpus.txt");
    let names: Vec<(&str, &str)> = apps
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter_map(|l| l.split_once(' '))
        .collect();
    let app_sets: &[(&str, &[&str])] = &[
        ("formicaria 0.55 (EasyPrivacy, whole engine)", &[]),
        ("now: EasyPrivacy's whole-site rules", &["easyprivacy"]),
        ("now: + DuckDuckGo's app list", &["easyprivacy", "ddg-apps"]),
        ("now: + Peter Lowe's (the app blocker's set)", &["easyprivacy", "ddg-apps", "pgl"]),
    ];
    println!("\n| App blocker | trackers refused | ordinary refused |\n|---|---|---|");
    for (name, keys) in app_sets {
        let refused: Box<dyn Fn(&str) -> bool> = if keys.is_empty() {
            let texts = read(dir, &["easyprivacy"]);
            let b = Blocker::new(&texts.iter().map(String::as_str).collect::<Vec<_>>());
            Box::new(move |n| b.host_blocked(n))
        } else {
            let mut h = HostSet::new();
            for t in read(dir, keys) {
                h.add(&t);
            }
            Box::new(move |n| h.contains(n))
        };
        let (mut t, mut tn, mut wrong) = (0, 0, Vec::new());
        for (kind, n) in &names {
            if *kind == "tracker" {
                tn += 1;
                t += usize::from(refused(n));
            } else if refused(n) {
                wrong.push(*n);
            }
        }
        let on = names.iter().filter(|(k, _)| *k == "ordinary").count();
        println!(
            "| {name} | {t} of {tn} | {} of {on}{} |",
            wrong.len(),
            if wrong.is_empty() { String::new() } else { format!(" ({})", wrong.join(", ")) }
        );
    }
}
