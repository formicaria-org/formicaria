//! **Step B0** (`web-media-privacy-plan-2026-11-17.md`): what the engine costs with real lists.
//! `cargo run --release -p fm-block --example measure -- <list.txt>...` — memory (resident, Linux),
//! the time to build and to load a kept engine, its size kept, and the time per request.
use std::time::Instant;

fn rss_kb() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmRSS:"))
                .map(|l| l.split_whitespace().nth(1).unwrap_or("0").parse().unwrap_or(0))
        })
        .unwrap_or(0)
}

fn main() {
    let texts: Vec<String> =
        std::env::args().skip(1).map(|p| std::fs::read_to_string(p).unwrap()).collect();
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
    let before = rss_kb();
    let t = Instant::now();
    let b = fm_block::Blocker::new(&refs);
    let build = t.elapsed();
    let after = rss_kb();
    let kept = b.save();
    drop(b);
    let base = rss_kb();
    let t = Instant::now();
    let b = fm_block::Blocker::load(&kept).unwrap();
    let load = t.elapsed();
    let loaded = rss_kb();
    let urls = [
        ("https://www.google-analytics.com/analytics.js", "script"),
        ("https://securepubads.g.doubleclick.net/tag/js/gpt.js", "script"),
        ("https://news.example/static/app.js", "script"),
        ("https://cdn.example/img/photo.jpg", "image"),
        ("https://connect.facebook.net/en_US/fbevents.js", "script"),
        ("https://news.example/api/comments?id=1", "xmlhttprequest"),
    ];
    let n = 20_000;
    let t = Instant::now();
    let mut blocked = 0;
    for i in 0..n {
        let (u, k) = urls[i % urls.len()];
        if b.check(u, "https://news.example/story", k) {
            blocked += 1;
        }
    }
    let per = t.elapsed() / n as u32;
    println!("lists: {} rules lines", texts.iter().map(|t| t.lines().count()).sum::<usize>());
    println!("build: {build:?}, memory +{} MB", (after.saturating_sub(before)) / 1024);
    println!(
        "kept engine: {} MB; load: {load:?}, memory +{} MB",
        kept.len() / 1024 / 1024,
        loaded.saturating_sub(base) / 1024
    );
    println!("per request: {per:?} ({blocked} of {n} refused)");
    for (u, k) in urls {
        println!(
            "  {} {u}",
            if b.check(u, "https://news.example/story", k) { "refused" } else { "allowed" }
        );
    }
}
