# 2026-07-20 — Capture on a real phone, and a device that eats every diagnostic

Continues `2026-07-19-media-on-the-phone.md`, which built the byte path but ended with capture
**unverified on real hardware** — the emulator has no camera and the owner needed their phone.

## What is now verified on the phone

The capture pipeline works end to end **except display**:

| Step | State |
|---|---|
| Capture menu → system camera Intent | works — the camera opens, no plugin, no declared permission |
| Photo bytes → `POST fmblob://…/ingest` | works, after two fixes below |
| `ingest_bytes` → blob in the vault | works — a hash comes back and the reference is inserted |
| Reference → rendered image | **fails** — the note shows the filename as text |

Two fixes were needed to get the bytes moving, both Android-specific and neither visible on the
emulator:

- **`InvokeBody::Raw` is unsupported on Android.** The raw-IPC-body design from the plan cannot
  work there at all; ingest moved to a `POST` on the `fmblob://` protocol handler, which is the
  same transport blobs already stream out over. One transport, both directions.
- **The CORS preflight was unanswered.** The custom scheme is rewritten to an `http` origin, so
  the page's own request to its own handler is *cross-origin*; without an `OPTIONS` arm every
  capture died as `TypeError: Failed to fetch`.

Also corrected: **wry only honours `capture` for `image/*` and `video/*`**, so "Record audio"
opened a file browser rather than a recorder. Relabelled to "Choose an audio file" — the button
now describes what it does instead of promising what the platform will not give.

## The lesson: this device has no diagnostic channel but the screen

Three channels were tried on the owner's phone and **two are silently dead**:

| Channel | Result |
|---|---|
| `eprintln!` / stdout | never reaches logcat from a Tauri Android shell |
| `console.warn` from the WebView | **nothing** — logcat carries the native `ca-bundle:` lines from the same run and not one JS line |
| The app's own UI | works — this is how the CA bundle status was read all through the TLS work |

A `console.warn` diagnostic was built, signed, installed (MD5-verified) and produced **zero
output**. That is a whole round trip — build, sign, install, ask the owner to reopen a note — spent
on a channel that cannot report.

**So the reason an asset fails to render is now part of the resolver's contract, not a log line.**
`AssetResolver` returns `ResolvedAsset | AssetFailure | null`, and the placeholder renders
`<label> — <reason>`. This is better permanently, not just for debugging: a placeholder that reads
`photo.jpg` is identical whether the bytes are absent, unreadable, or the lookup threw — three
problems with three different fixes, previously indistinguishable on the one surface anyone can
see.

## Two hypotheses killed by reading, not guessing

Both were offered to the owner before being checked, and both were wrong:

1. **"The blob and the note are in different vaults."** `dispatch.rs`'s `asset_status` arm
   searches **every** registered vault; a mismatch cannot produce `has_blob: false`.
2. **"The reference format is wrong."** `parse_ref` accepts `asset:`, `sha256:` *and* `sha256-`,
   which is exactly what the capture writes.

Reading either one takes a minute. This is the same failure as the five wrong TLS diagnoses on
2026-07-19 — a hypothesis stated with confidence before the code that settles it was opened.

## Also this session

- **A blank window on the phone**, on any vault containing an asset. `convertFileSrc` threw
  during render; the emulator vault was empty so it never reproduced there. `blobBase` is now its
  own dependency-free module that cannot throw and falls back to `fmblob://localhost/`.
- **The Timeline filed notes under the wrong day.** It cut the ISO string (`created.slice(0,10)`),
  which is UTC, so at UTC+8 everything written after midnight appeared under "Yesterday" for eight
  hours. Now `ymd(new Date(...))`, which is local. Pre-existing, unrelated to mobile.

## Still open — first thing tomorrow

**The captured photo does not render.** A build carrying the on-screen reason is installed
(MD5 `d0796a29b271338e796e36b089b36a4e`, verified against the phone). The placeholder will now
say which of these it is:

- `no bytes in vault "…"` — ingest returned a hash but nothing landed
- `the vault has this blob but it read back empty`
- any other text — `asset_status` threw, and the text is the error

One look at the note answers it. **Do not theorise before reading that line.**
