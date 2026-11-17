# Blocking on the phone: what the engine costs (step B0, 2026-11-17)

Plan: `web-media-privacy-plan-2026-11-17.md`, part B. The question was whether ad blocking could be
on by default on the phone, as the computer's precedent (uBlock Origin: **+66 % memory**, a fresh
profile compiling its lists; `decisions.md` 2026-11-15) said it could not be there.

## What was measured

`cargo run --release -p fm-block --example measure -- <lists>` on the development computer (x86-64,
release build), with the lists as published that day:

| Lists | Rule lines | Build from text | Memory, built | Kept engine | Load kept | Memory, loaded | Per request |
|---|---|---|---|---|---|---|---|
| EasyPrivacy (trackers) | 57,450 | 30 ms | +4 MB | 2 MB | 1.2 ms | +2 MB | 1.9 µs |
| EasyPrivacy + EasyList (ads) | 134,700 | 61 ms | +9 MB | 5 MB | 2.2 ms | +5 MB | 1.9 µs |

Six representative requests from a news page (analytics, an ad tag, a social pixel, the page's own
script, a picture, its own API): the three third-party trackers and ads refused, the page's own
three allowed, with both lists.

## What it means

- **The engine is cheap.** A few MB and a couple of microseconds a request; a phone CPU several
  times slower still keeps a request well under 0.1 ms, against network times of tens of ms.
- **The build runs once a week** (when the lists are refreshed); every other start loads the kept
  engine in a few ms.
- **So ad blocking is on by default on the phone** (`privacy.rs`), and stays off by default on the
  computer, where it means installing uBlock Origin into Firefox.

## Not yet measured

- **On a phone itself**: no device was attached. The figures above are the development
  computer's; a phone measure (memory of the app with five web tabs, blocking on and off) belongs
  in this file when one is.
- The cost of the hide rules in a page (the names a page uses, asked again at most once a second as
  it changes).
