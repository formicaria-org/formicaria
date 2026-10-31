//! **Where the food a person means lands in a food search, on the real list.**
//!
//! `food-values-plan-2026-10-30.md`, step 1: a change to how `FoodPack::search` orders its results
//! must raise the meant food and move none down. The words and the food each one means are in
//! `food_search_ranks.tsv`; the numbers before and after are in
//! `docs/context/food-search-ranks-2026-10-31.md`.
//!
//! Needs the real food list, so it is ignored by default:
//!
//! ```sh
//! FM_FOODS=target/packs/foods.sqlite pixi run cargo test -p fm-core --test food_search_ranks -- --ignored --nocapture
//! ```
//!
//! `FM_FOODS_DUMP=1` prints each word's first ten results too, to choose the meant food from.

use fm_core::pack::FoodPack;
use rusqlite::{Connection, OpenFlags};
use std::collections::BTreeSet;
use std::path::Path;

/// How far down the meant food may be and still count as found on the first screen.
const FIRST_SCREEN: usize = 5;

#[test]
#[ignore = "needs FM_FOODS=<foods.sqlite>"]
fn the_meant_food_and_its_rank() {
    let path = std::env::var("FM_FOODS").expect("FM_FOODS=<the food list> is needed");
    let pack = FoodPack::open(Path::new(&path)).expect("a food list");
    let db = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let dump = std::env::var("FM_FOODS_DUMP").is_ok();
    let set = include_str!("food_search_ranks.tsv");

    let (mut first, mut screen, mut missing, mut words) = (0, 0, 0, 0);
    for line in set.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#')) {
        let mut cols = line.split('\t');
        let (q, meant) = (cols.next().unwrap(), cols.next().unwrap_or(""));
        // A food and every food in its group count: a search shows a group once, under any member.
        let mut ok = BTreeSet::new();
        for code in meant.split(',').map(str::trim).filter(|c| !c.is_empty()) {
            ok.insert(code.to_string());
            let mut stmt = db
                .prepare(
                    "SELECT g.code FROM food f JOIN same s ON s.food = f.id
                       JOIN same t ON t.first = s.first JOIN food g ON g.id = t.food
                      WHERE f.code = ?1",
                )
                .unwrap();
            let same = stmt.query_map([code], |r| r.get::<_, String>(0)).unwrap();
            ok.extend(same.map(Result::unwrap));
        }
        let hits = pack.search(q, 30).unwrap();
        let rank = hits.iter().position(|h| ok.contains(&h.id)).map(|i| i + 1);
        words += 1;
        match rank {
            Some(1) => first += 1,
            None => missing += 1,
            _ => {}
        }
        if rank.is_some_and(|r| r <= FIRST_SCREEN) {
            screen += 1;
        }
        let shown = rank.map_or("—".to_string(), |r| r.to_string());
        println!("{q}\t{shown}");
        if dump || meant.is_empty() || rank.is_none() {
            for h in hits.iter().take(10) {
                println!(
                    "    {:<16} {}{}",
                    h.id,
                    h.name,
                    if h.alt.is_empty() { String::new() } else { format!(" · {}", h.alt) }
                );
            }
        }
    }
    println!(
        "\n{words} words: first {first}, in the first {FIRST_SCREEN} {screen}, not in 30 {missing}"
    );
}
