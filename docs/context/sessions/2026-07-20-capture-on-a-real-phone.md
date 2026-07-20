# 2026-07-20 — Capture on a real phone, and a device that eats every diagnostic

Continues `2026-07-19-media-on-the-phone.md`, which built the byte path but ended with capture
**unverified on real hardware** — the emulator has no camera and the owner needed their phone.

**Media on Android now works end to end.** Attach a file, it lands in the vault, the note renders
it. That took most of the day and two false summits; the account below is in the order it
happened, because the wrong turns are the useful part.

| Step | State |
|---|---|
| Capture menu → system camera Intent | works — no plugin, no declared permission |
| File bytes → the app | works, **via `fm_ingest` over IPC** — the POST design could never work, see below |
| `ingest_bytes` → blob in the vault | works — hash verified against `sha256sum` |
| Reference → rendered image | **works**, verified on Android |

Two Android-specific fixes were needed even to get a *request* through, neither visible on the
emulator at the time:

- **`InvokeBody::Raw` is unsupported on Android.** The raw-IPC-body design from the plan cannot
  work there at all, so ingest moved to a `POST` on the `fmblob://` protocol handler.
  **This was also wrong**, for a deeper reason — see "the bytes never left the page".
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

## Solved: the bytes never left the page

**`fetch(url, { body: file })` sends nothing through a custom scheme on Android**, and nothing
errors. `ingest` hashed zero bytes, stored the empty blob, and returned a reference — so **every
photo ever taken produced the same one**:

```
asset:sha256-e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
```

That constant is the SHA-256 of the empty string, and recognising it is what finally identified
this after two wrong hypotheses about vaults and reference formats. The lesson is cheap and
general: **a content-addressed store makes its own failures legible** — the hash of nothing is a
fixed, recognisable value, and it was sitting in the note the whole time.

### Why the transport had to change

This is a **platform limit, not a bug to fix**. wry intercepts requests through
`WebViewClient.shouldInterceptRequest(view, request: WebResourceRequest)`, and Android's
`WebResourceRequest` exposes the URL, the method and the headers — **and no body**. There is no
accessor for one; wry reads none because none exists. A POST arrives with its body silently
dropped. Combined with Tauri's own *"On Android, `InvokeBody::Raw` is not supported"*, both binary
doors are shut, and the only remaining transport is a **JSON string**.

So ingest moved to `fm_ingest`, a real second Tauri command taking the file base64-encoded. That
costs a third more bytes and a few copies, against media that did not arrive at all. `MAX_INGEST`
is 48 MB — comfortably above any phone photo, honestly below video, and it **says so** rather than
running out of memory. Chunking is the real answer for video and is a separate piece.

The `fmblob://` scheme keeps the job it is good at: blobs coming *out*, streamed, seekable.

### Three failures on the way, each worth one line

1. `nativeInvoke` wraps everything into the single `fm` command, so calling it with `fm_ingest`
   became `dispatch("fm_ingest")` → *"unknown command: fm_ingest"*. Real shell commands need a
   direct call; `shellInvoke` is now that, and `nativeInvoke` is one line on top of it.
2. Shell commands return JSON **text**. Forgetting to parse hands back a `string` that
   type-checks as anything and dies at the first property access.
3. The first fix — reading the `File` into an `ArrayBuffer` — was reasonable and **did not help**,
   because the body was never the problem; the transport was.

### Verified, on Android, end to end

A JPEG pushed to the emulator, picked through the real system picker, ingested, and **rendered in
the note**. The reference's hash matches `sha256sum` of the file byte for byte.

## Guards so this cannot recur quietly

- **`ingest` refuses an empty body**, in `dispatch`, where every frontend crosses. Attaching a
  genuinely empty file gains nothing; accepting one *silently* hid a broken byte path behind a
  success message for days. The message says it is a transport problem, not a bad file.
- **The client refuses to send an empty body for a non-empty file** — the contradiction that
  names the bug at the moment it appears.
- `capture_round_trip.rs` covers the round trip through `dispatch` and pins the empty-blob
  refusal, including that no blob is left behind.

## Still open

- **In-app audio recording** (`getUserMedia` + `MediaRecorder`, the `RECORD_AUDIO` permission and
  wry's permission plumbing). The picker ships a working feature meanwhile.
- **Video on Android.** Above `MAX_INGEST` it is refused with an explanation. Chunked ingest would
  lift it, and the storage question behind it — a phone vault holds the only copy, and restic is
  not available there — is still unanswered.
