//! The Android shell: a window, the platform seam, and nothing else.
//!
//! **Everything it answers goes to [`fm_app::dispatch`]** — the same door `fm-serve` uses. That
//! is ruling 1 of the mobile design, and the whole reason a second frontend costs a file rather
//! than a fork: the `match` over command names, the vault list, and the lock discipline all live
//! in `fm-app`, so this crate has no opinion about any of them.
//!
//! There is essentially one command — [`fm`], taking a command name and a JSON blob, because the
//! wire contract already *is* "name plus JSON" — `ui/src/lib/ipc.ts` posts precisely that to
//! `/api/<cmd>`. Enumerating thirty wrapper functions here would be thirty places to forget one.
//! [`fm_ingest`] is the one unavoidable second, because Android cannot carry bytes any other way.
//!
//! **Both are `#[tauri::command(async)]`, and that is not a style choice** — a blocking command
//! parks the WebView's JS thread for its whole duration on this platform. See [`fm`] for the
//! three-fact chain, and `ci/checks.sh` for the guard that keeps a new command from forgetting it.

use std::sync::Arc;

use fm_app::{dispatch, App, Host};
use fm_core::ColdStart;

// Behind `agent_shell` — the `agent` feature (default on) **and** Android. A
// `--no-default-features` build compiles none of it, so a notes-only APK never links the model
// runner: the mobile half of "the core never knows the agent exists." See Cargo.toml.
//
// The Android half of the gate is not a second opinion about the feature, it is Route C
// (`decisions.md#track-m`, *iOS ships agent-free*) made structural: an iOS build with default
// features would otherwise fail to compile on `native_lib_dir`. `build.rs` derives the cfg and
// explains why Cargo cannot; every arm below reads `agent_shell`, never the raw feature.
#[cfg(agent_shell)]
mod agent;
#[cfg(update_shell)]
mod update;

/// Android's answer to "hand this file to whatever owns it" is an `Intent`, which needs the JVM;
/// iOS's is a `UIDocumentInteractionController`. Wiring either is a later milestone
/// (`tauri-plugin-opener`); until then this says so rather than pretending, because a silent no-op
/// here looks like a broken PDF to a user.
///
/// **Named for the shell, not for one platform.** It was called `AndroidHost` until 2026-09-03;
/// nothing in it was ever Android-specific, and the name was one of the places an iOS port would
/// have had to argue with a label rather than with code.
struct MobileHost;

/// The last startup failure, if there was one — and the lock that serialises attempts to fix it.
///
/// A startup failure used to be a `?` out of the `setup` hook, i.e. a panic on the thread Tauri
/// runs `run()` on. The Android Activity does not die with it: `WryActivity.onCreate` has long
/// since returned, so the webview stays up, the IPC replies stop, and the page waits forever on a
/// `list_vaults` that will never answer. Holding the reason here instead means every command can
/// hand the UI a sentence to render.
static BOOT: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// Ensures the after-the-store work happens exactly once however many times boot is attempted.
static FINISH: std::sync::Once = std::sync::Once::new();

/// Open the vaults, publish them, and kick off everything that comes after — or return why not.
///
/// **Callable more than once, and that is the point.** The first draft recorded a failure in a
/// `OnceLock` and stopped there, which froze the message for the life of the process: the UI's
/// "Try again" button re-invoked `list_vaults`, `list_vaults` found no managed state, and answered
/// the same stale sentence — a button that provably could not help, in front of an app that a
/// single transient failure (an `index.sqlite` busy for one launch, a read that failed under memory
/// pressure) had bricked until the user found the app switcher. Since `manage` works on an
/// `AppHandle` at any time, a retry can genuinely retry.
///
/// `catch_unwind` because the shell must survive a panic *inside* the open — rusqlite, a descriptor
/// parse, serde — for the same reason: a panicked thread leaves a webview asking a backend that
/// will never speak, and the phone has no other channel to say so.
fn boot(handle: &tauri::AppHandle) -> Result<Arc<App>, String> {
    use tauri::Manager;
    // **Refuse before opening anything.** If `setup` could not configure the paths, opening the
    // vaults would *succeed* — with none — and hand the user a first-run form that cannot save.
    // Saying so is the whole fix; the app cannot make the platform produce a data directory.
    if let Some(Err(why)) = PATHS.get() {
        return Err(why.clone());
    }
    // **`TrustIndex`, and this is the one place that claim can honestly be made.**
    //
    // Opening a vault rebuilt its whole FTS index from every file on disk, every time. On a
    // desktop that is a startup cost you pay once a day; Android kills backgrounded apps
    // constantly, so here it was the cost of *every* relaunch — the app's slowest moment, all day,
    // with a perfectly good `index.sqlite` sitting beside it.
    //
    // The reason it is safe here and not on the desktop: reconciliation is by mtime, so a tool
    // that rewrites a file while *preserving* its mtime is invisible to it. `cp -p`, `rsync -a`
    // and a restic restore all do that. A phone has none of them — no shell, no restic, and
    // libgit2 writes files fresh — so no such writer exists on this platform. That is a fact about
    // the device, which is why the shell says it rather than the store assuming it. See
    // `fm_core::ColdStart`, and `fm-core/tests/cold_start.rs`, which asserts the divergence
    // instead of pretending it is not there.
    //
    // It fails safe in both directions anyway: an index from another schema, or one whose rebuild
    // was interrupted (`SIGKILL` — there is no shutdown hook to trust), carries no completion
    // marker and is rebuilt in full.
    let loaded =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| App::load_with(ColdStart::TrustIndex)))
            .map_err(|_| "opening the vaults panicked — see logcat for the backtrace".to_string())
            .and_then(|r| r.map_err(|e| format!("could not open vaults: {e}")));
    let (fm_app, skipped) = match loaded {
        Ok(v) => v,
        Err(e) => {
            log::error!("{e}");
            return Err(e);
        }
    };
    if !skipped.is_empty() {
        // Same discipline as the desktop: a note that could not be read is named, never
        // swallowed. The UI surfaces these on the heartbeat.
        log::warn!("unreadable notes: {}", skipped.join("; "));
    }
    let app_state = Arc::new(fm_app);
    // **Published before anything else runs.** From here the `fm` command can answer, so every
    // millisecond spent after this point is a millisecond the UI has data for.
    handle.manage(app_state.clone());
    // One line, and it is load-bearing: it is what `ci/android-smoke.sh` greps to prove the vaults
    // opened *before* the optional work — the ordering whose absence made the first open gray.
    // (On the owner's own phone even this is invisible: MIUI suppresses our tag, which is why the
    // same fact has to reach the screen — see known-issues.md.)
    log::info!("vaults ready: {} vault(s)", app_state.configs().map(|c| c.len()).unwrap_or(0));

    // The rest is off the critical path, in this order, on one thread: the trust store first (the
    // agent's downloads need it), then the model. Both used to sit *above* the open, where they
    // delayed the first thing the user sees to buy nothing — neither is needed to read a note.
    let after = handle.clone();
    let for_agent = app_state.clone();
    FINISH.call_once(move || {
        std::thread::spawn(move || {
            // Android only: iOS's libgit2 links SecureTransport, not OpenSSL, so there is no
            // bundle to install and the two Android certificate directories do not exist. Left
            // ungated it would log `ca-bundle: <error>` on every iOS launch — a startup error line
            // that is not one, in the one channel the Simulator smoke test has to read.
            #[cfg(target_os = "android")]
            install_ca_bundle(&after);
            // A notes-only build (`--no-default-features`) compiles no agent at all, so the store
            // it would have been handed is deliberately dropped here. Same for the handle on a
            // platform that has neither the trust store nor the agent — iOS today — where this
            // thread has genuinely nothing to do and the alternative is an `unused_variables`
            // warning in the one log an iOS run can be read from.
            #[cfg(not(agent_shell))]
            let _ = for_agent;
            #[cfg(not(target_os = "android"))]
            let _ = &after;
            #[cfg(agent_shell)]
            {
                use tauri::Manager;
                if let Ok(dir) = after.path().app_data_dir() {
                    if let Err(e) = agent::start(for_agent, dir.join("agents")) {
                        log::info!("study agent not started: {e}");
                    }
                }
            }
        });
    });
    Ok(app_state)
}

/// The vault store, or a sentence saying why not — retrying the open if it never succeeded.
///
/// **Deliberately not a `tauri::State<Arc<App>>` parameter.** That extractor answers an unmanaged
/// state with *"state not managed for field `app` on command `fm`. You must call `.manage()`"* — a
/// message about our own wiring, shown to someone holding a phone. Resolving it by hand costs a few
/// lines and lets the shell recover instead of explaining itself.
fn vault_state(handle: &tauri::AppHandle) -> Result<Arc<App>, String> {
    use tauri::Manager;
    if let Some(state) = handle.try_state::<Arc<App>>() {
        return Ok(state.inner().clone());
    }
    // Serialise the attempts: the webview, the heartbeat and the agent all call in, and two
    // concurrent `App::load()`s would open — and reindex — every vault twice.
    let mut last = BOOT.lock().map_err(|_| "the startup lock is poisoned".to_string())?;
    // Re-check under the lock: whoever we queued behind may have succeeded.
    if let Some(state) = handle.try_state::<Arc<App>>() {
        *last = None;
        return Ok(state.inner().clone());
    }
    match boot(handle) {
        Ok(app) => {
            *last = None;
            Ok(app)
        }
        Err(e) => {
            *last = Some(e.clone());
            Err(e)
        }
    }
}

impl Host for MobileHost {
    fn open_external(&self, path: &std::path::Path) -> Result<(), String> {
        Err(format!(
            "opening {} outside the app needs the platform opener, which this build does not \
             have yet",
            path.display()
        ))
    }
}

/// The single command. `cmd` is the name `ipc.ts` would have POSTed; `args` is the same JSON
/// body. The reply is the raw bytes `dispatch` produced, handed back as a string because every
/// command the shell can reach answers JSON (blob bytes go over a protocol handler, not IPC —
/// ruling 7).
/// Blob bytes, over a URI scheme rather than the command channel — **the "ruling 7" path that
/// was described in this file's own header and never written.**
///
/// Without it media does not work on this platform in either direction: `resolve_asset` answers
/// with raw bytes, and [`fm`] runs every reply through `String::from_utf8`, so a JPEG fails to
/// decode; while `assetUrl` points at `/api/blob/…`, an HTTP route that exists only in
/// `fm-serve` and never on a phone.
///
/// A protocol handler is the right shape rather than base64 over IPC: it **streams**, so a video
/// is not held in memory twice on the way to the screen, and the webview can seek within it.
///
/// `fmblob://localhost/<url-encoded reference>` — the reference is whatever the note wrote
/// (`sha256:…` or `asset:sha256-…`), resolved by the same command the desktop uses, so there is
/// one implementation of what a reference means.
fn blob_response(
    app: &tauri::AppHandle,
    method: &str,
    uri: &str,
    body: &[u8],
    range: Option<String>,
) -> tauri::http::Response<Vec<u8>> {
    use tauri::http::{Response, StatusCode};
    use tauri::Manager; // `try_state` lives on the trait, not on `AppHandle` itself
    let not_found = || {
        Response::builder().status(StatusCode::NOT_FOUND).body(Vec::new()).unwrap_or_default()
    };
    // Everything after the host: `fmblob://localhost/<ref>` → `<ref>`.
    let Some(rest) = uri.split_once("://").and_then(|(_, r)| r.split_once('/')).map(|(_, r)| r)
    else {
        return not_found();
    };
    let (path, query) = rest.split_once('?').unwrap_or((rest, ""));
    // **The `query` above was parsed and never read** — the compiler said so on every build. It was
    // the unfinished half of thumbnails: the desktop route discarded its query string for the same
    // reason, so a thumbnail could be generated and served by a command but never *linked to*.
    // `?kind=thumb` fills the slot. Anything else means the full blob; an unknown kind is not an
    // error, just not a request for the small copy.
    let kind = if query.split('&').any(|kv| kv == "kind=thumb") { "thumb" } else { "full" };

    // **The preflight is still answered, though nothing this app writes needs it now.** On
    // Android the page is served from `http://tauri.localhost` while this handler answers on
    // `http://fmblob.localhost` — a *different origin* — so any `fetch` here that is not a simple
    // request gets an `OPTIONS` first and stalls until something replies. Nothing did once, and
    // it surfaced in the page as a bare `TypeError: Failed to fetch` with no status to explain
    // it. Four lines to keep that from being rediscovered.
    if method.eq_ignore_ascii_case("OPTIONS") {
        return tauri::http::Response::builder()
            .status(StatusCode::NO_CONTENT)
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Allow-Methods", "GET, OPTIONS")
            .header("Access-Control-Allow-Headers", "*")
            .header("Access-Control-Max-Age", "86400")
            .body(Vec::new())
            .unwrap_or_else(|_| not_found());
    }

    let Some(state) = app.try_state::<Arc<App>>() else { return not_found() };

    // **Ingest does *not* ride this scheme, and cannot.** A POST here reaches this function
    // with its body silently dropped: wry intercepts through
    // `WebViewClient.shouldInterceptRequest(view, request: WebResourceRequest)`, and Android's
    // `WebResourceRequest` has no body accessor — there is nothing for wry to pass on. This
    // handler served `POST /ingest` for a while and stored every photo as zero bytes while
    // reporting success. Ingest lives on the `fm_ingest` IPC command; see its note.
    //
    // This scheme stays the way blobs come *out*.
    //
    // **The comment here used to claim "a GET streams, and `<video>` can seek without the file
    // ever being held whole in memory". None of that was true** (corrected 2026-09-04), and the
    // "streams" half is not even achievable: `register_uri_scheme_protocol` hands back a
    // `Response<Vec<u8>>` and Tauri offers no streaming body type at this seam. What *is*
    // achievable, and what actually fixes the memory problem, is honouring `Range` — so the peak
    // allocation is **one requested window**, not one file. Say that, and not more than that.
    //
    // This stopped being a latent cost on 2026-09-04, when chunked ingest removed the phone's
    // upload ceiling: the files this path must serve are now unbounded.

    let reference = percent_decode(path);
    // `blob_path_of_kind` rather than `dispatch("resolve_asset")`: it is `pub` for exactly this —
    // "for a transport that wants to stream the bytes itself rather than take them through
    // `Output::Bytes`" — and `fm-serve/src/blob.rs` already uses it. Not a breach of the
    // every-command-through-`dispatch` rule; it is the sanctioned transport-side door, and it is
    // what makes the phone and the desktop share one resolver, one fallback and one entitlement
    // check.
    let path_of = fm_app::dispatch::blob_path_of_kind(
        &state,
        &fm_app::Scope::All,
        &reference,
        kind == "thumb",
    );
    let blob = match path_of {
        Ok(p) => p,
        Err(e) => {
            log::warn!("blob {reference}: {e}");
            return not_found();
        }
    };
    let total = match std::fs::metadata(&blob) {
        Ok(m) => m.len(),
        Err(e) => {
            log::warn!("blob {reference}: {e}");
            return not_found();
        }
    };
    // Sniffed from the bytes, as the desktop does: the blob store is content-addressed and keeps
    // no MIME beside them. The old hardcoded `application/octet-stream` is why `inline_safe` could
    // not have worked here even if it had been called.
    let ctype = fm_core::ingest::sniff_mime(&blob)
        .unwrap_or_else(|| "application/octet-stream".to_string());
    let reply = fm_app::wire::blob_reply(total, &ctype, range.as_deref());

    let body = match reply.range {
        None => Vec::new(),
        Some((start, end)) => {
            use std::io::{Read, Seek, SeekFrom};
            let mut f = match std::fs::File::open(&blob) {
                Ok(f) => f,
                Err(e) => {
                    log::warn!("blob {reference}: {e}");
                    return not_found();
                }
            };
            // **Seek and read exactly the window — never `std::fs::read`.** This is the whole
            // memory fix: a 200 MB video answered in 1 MB slices allocates 1 MB at a time.
            if f.seek(SeekFrom::Start(start)).is_err() {
                return not_found();
            }
            let mut buf = vec![0u8; (end - start + 1) as usize];
            match f.read_exact(&mut buf) {
                Ok(()) => buf,
                Err(e) => {
                    log::warn!("blob {reference}: {e}");
                    return not_found();
                }
            }
        }
    };

    let mut builder = Response::builder()
        .status(reply.status)
        .header("Content-Type", &ctype)
        // **Advertised, or a media element will not even try to seek.** A `<video>` that sees no
        // `Accept-Ranges` downloads the whole file before it plays a frame.
        .header("Accept-Ranges", "bytes")
        // The same three the desktop sets, and for a reason that is not desktop-specific: a blob
        // is reachable as a same-origin URL, and **blobs arrive from collaborators through the
        // merge driver**. Navigating straight to someone else's file must not run it.
        .header("X-Content-Type-Options", "nosniff")
        .header(
            "Content-Security-Policy",
            "default-src 'none'; img-src 'self' blob: data:; media-src 'self' blob:; \
             object-src 'none'; script-src 'none'; sandbox",
        )
        .header("Access-Control-Allow-Origin", "*");
    if !reply.inline {
        builder = builder.header("Content-Disposition", "attachment");
    }
    if let Some(cr) = reply.content_range {
        builder = builder.header("Content-Range", cr);
    }
    // A missing blob is the ordinary case for a vault whose media has not synced — every failure
    // path above answers `not_found()`, and the note renders a placeholder, which is the same
    // thing the desktop does.
    builder.body(body).unwrap_or_else(|_| not_found())
}

/// Minimal percent-decoding for the one place a reference crosses a URL.
///
/// **Lives in `fm_app::wire`**, not here. This crate is excluded from the workspace, so anything
/// written in it is never compiled by `cargo test --workspace` and never tested — which is how
/// `b64_decode`, the function every phone photo passes through, reached production with no test
/// at all. The pure encodings moved to where the tests already run; what stays here is what
/// genuinely needs Tauri.
use fm_app::wire::percent_decode;

/// **Ingest, the only way Android allows.**
///
/// The bytes arrive base64 in a JSON argument, which is wasteful and is nonetheless the only
/// transport that exists here. Two doors were tried and both are closed by the platform:
///
/// - **Tauri's raw IPC body.** Tauri's own docs: *"On Android, `InvokeBody::Raw` is not
///   supported. The enum will always contain `InvokeBody::Json`."*
/// - **A POST to the custom scheme.** wry intercepts through
///   `WebViewClient.shouldInterceptRequest(view, request: WebResourceRequest)`, and Android's
///   `WebResourceRequest` exposes the URL, the method and the headers — **and no body**. There is
///   no accessor to add; wry reads none because none exists. A POST reaches the handler with its
///   body silently dropped, which is exactly how every photo taken on a phone came to be stored
///   as zero bytes: `ingest` hashed nothing, and every capture produced the same reference,
///   `e3b0c442…b855`, the SHA-256 of the empty string.
///
/// So base64 it is: a third larger and copied a few times, against media that does not arrive at
/// all. The size ceiling that costs is real and is stated to the user rather than discovered as a
/// crash — see `MAX_INGEST` below.
/// **`(async)`, and that word is the difference between a usable app and a frozen one.** See the
/// note on [`fm`] — this command is the worst case, because decoding a photo and writing a blob is
/// the longest thing the phone ever does inside one call.
#[tauri::command(async)]
fn fm_ingest(
    name: String,
    vault: String,
    data: String,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let app = vault_state(&app_handle)?;
    let bytes = b64_decode(&data).ok_or_else(|| format!("{name}: could not decode the file"))?;
    let args = serde_json::json!({ "name": name, "vault": vault });
    let out = dispatch("ingest", &args, &bytes, &app, &MobileHost).inspect_err(|e| {
        log::error!("ingest: {e}");
    })?;
    String::from_utf8(out.into_bytes()).map_err(|e| e.to_string())
}

/// **One slice of a chunked upload.** The bytes arrive base64, exactly as [`fm_ingest`]'s do and
/// for the same reason — Android has no other door — but the file is sliced first, so what is
/// held in memory here is one chunk instead of the whole thing.
///
/// That is the whole change, and it is what lifts the 16 MB ceiling: the limit was never about
/// how large an attachment ought to be, it was the point at which base64-in-a-JSON-argument
/// stopped fitting. A slice is bounded, so the file need not be.
///
/// **`(async)` for the same reason every command here is**: see [`fm`]. A blocking one parks the
/// page's JS thread, and an upload is exactly the situation where the user is watching.
#[tauri::command(async)]
fn fm_ingest_chunk(
    session: String,
    seq: u32,
    vault: String,
    data: String,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let app = vault_state(&app_handle)?;
    let bytes = b64_decode(&data)
        .ok_or_else(|| format!("upload {session}: chunk {seq} could not be decoded"))?;
    let args = serde_json::json!({ "session": session, "seq": seq, "vault": vault });
    let out = dispatch("ingest_chunk", &args, &bytes, &app, &MobileHost).inspect_err(|e| {
        log::error!("ingest_chunk: {e}");
    })?;
    String::from_utf8(out.into_bytes()).map_err(|e| e.to_string())
}

/// Assemble the session and write the asset note. No bytes cross here — they are already on disk.
#[tauri::command(async)]
fn fm_ingest_finish(
    session: String,
    name: String,
    vault: String,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let app = vault_state(&app_handle)?;
    let args = serde_json::json!({ "session": session, "name": name, "vault": vault });
    let out = dispatch("ingest_finish", &args, &[], &app, &MobileHost).inspect_err(|e| {
        log::error!("ingest_finish: {e}");
    })?;
    String::from_utf8(out.into_bytes()).map_err(|e| e.to_string())
}

/// Abandon an upload and reclaim its bytes now, rather than at the next boot sweep.
#[tauri::command(async)]
fn fm_ingest_cancel(
    session: String,
    vault: String,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let app = vault_state(&app_handle)?;
    let args = serde_json::json!({ "session": session, "vault": vault });
    let out = dispatch("ingest_cancel", &args, &[], &app, &MobileHost).inspect_err(|e| {
        log::error!("ingest_cancel: {e}");
    })?;
    String::from_utf8(out.into_bytes()).map_err(|e| e.to_string())
}

/// Standard base64 → bytes — **in `fm_app::wire`**, for the reason given on `percent_decode`
/// above: a function in this crate is a function no test can reach. Its guarantee (a mangled
/// payload fails rather than ingesting a prefix of the photo) is now pinned by tests that run in
/// `pixi run ci`.
use fm_app::wire::b64_decode;

/// **Every command on this platform must be `(async)`, or it freezes the screen.**
///
/// Three facts compose into one, and none of them is visible from this file alone:
///
/// 1. **Android never gets Tauri's async custom-protocol IPC.** `tauri`'s own `ipc-protocol.js`
///    opens with `const canUseCustomProtocol = osName !== 'android'`; the fallback is
///    `window.ipc.postMessage(JSON.stringify(payload))`.
/// 2. **`postMessage` is an `@JavascriptInterface` method, and wry runs the handler inline** —
///    `(ipc.handler)(request)` on the calling thread, no spawn, no channel. A JS→Java bridge call
///    is *synchronous*: the page's JS thread is parked until Java returns.
/// 3. **A plain `#[tauri::command]` is `ExecutionContext::Blocking`** — the macro runs the body
///    and resolves the promise before returning.
///
/// Together: the UI could not paint for the duration of *any* command. Not a slow one — any one.
/// That is why the phone presented as *freezing* rather than as merely slow, and why every other
/// cost on this platform (a full-corpus scan, an `ls-remote`, a full FTS rebuild) showed up as the
/// app locking up instead of as latency. The desktop never sees this: `fm-serve` is
/// thread-per-connection.
///
/// `(async)` on the *sync* function is the whole fix. The macro then emits
/// `resolver.respond_async_serialized(async move { … })`, which spawns onto the tokio runtime and
/// returns immediately — the JNI call returns, `postMessage` returns, JS keeps running.
///
/// **Deliberately `#[tauri::command(async)]` and not `async fn`.** There is no `.await` in either
/// body, so no `MutexGuard` can be held across one — the hazard that would make `App`'s
/// `std::sync::Mutex` unsound here simply cannot arise. Writing `async fn` would create a future
/// in which a later refactor *could* introduce one. The bound lands on the generated async block,
/// which captures only `String`/`Value`/`AppHandle`, all `Send + 'static`; `Arc<App>` is resolved
/// *inside* the body by `vault_state`, never passed across the boundary.
///
/// Two consequences to know about, both recorded in `known-issues.md`:
///   - the `pagehide` flush weakens from effectively-synchronous to fire-and-forget;
///   - replies can now interleave, so `NotePanel`'s note-loading effect needs the stale-response
///     guard it always should have had.
#[tauri::command(async)]
fn fm(
    cmd: String,
    args: serde_json::Value,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    // `agents`/`agent_status`/`set_agent` are **transport** concerns — the assistant's presence and its
    // on/off setting, the data behind the UI's `@`-picker and the Settings toggle. On the desktop
    // fm-serve answers these; the core `dispatch` never has them (that keeps the core agent-agnostic).
    // So the phone answers them the same way, here in the shell — never dispatched into the vault.
    if cmd == "agents" {
        #[cfg(agent_shell)]
        return Ok(serde_json::json!({ "agents": agent::online_agents() }).to_string());
        #[cfg(not(agent_shell))]
        return Ok("{\"agents\":[]}".to_string());
    }
    // **`agent_activity_poll` exists only in `fm-serve`, and the phone was asking anyway.**
    //
    // It is `fm-serve/src/agent.rs`'s own bookkeeping about a turn it is running — the core
    // `dispatch` has never had such an arm, and should not: it is a transport concern, like
    // `agents` above. So on the phone every poll fell through to `dispatch`, came back
    // `unknown command: agent_activity_poll`, and was `log::error!`-ed on the way out. While a
    // discussion was open that was a guaranteed-failing blocking round trip every 1.5 s — forty a
    // minute, each one parking the JS thread and filling logcat with the same line.
    //
    // Answering it here costs nothing and is *honest*: this shell runs no turn of its own, so it
    // has no activity to report. `{"active": false}` is the exact shape `ipc.ts` already falls
    // back to on error, so the UI is unchanged — it just stops paying for the error.
    if cmd == "agent_activity_poll" {
        return Ok("{\"active\":false}".to_string());
    }
    #[cfg(agent_shell)]
    if cmd == "agent_status" || cmd == "set_agent" || cmd == "set_transcribe" {
        use tauri::Manager;
        let dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?.join("agents");
        if cmd == "set_agent" {
            let on = args.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
            agent::set_running(vault_state(&app_handle)?, dir, on)?;
            return Ok("{\"ok\":true}".to_string());
        }
        if cmd == "set_transcribe" {
            // Persist only — whisper is chosen at the next agent start (a launch flag on a separate
            // process), so it applies when the assistant next starts, exactly like the desktop.
            let on = args.get("transcribe").and_then(|v| v.as_bool()).unwrap_or(false);
            agent::set_transcribe(&dir, on);
            return Ok("{\"ok\":true}".to_string());
        }
        // Every key fm-serve's `/api/agent_status` answers, because the same Settings panel reads
        // both — see `agent::availability`.
        let (installed, why, transcribe_available) = agent::availability();
        return Ok(serde_json::json!({
            "enabled": agent::is_enabled(&dir), "transcribe": agent::is_transcribe_enabled(&dir),
            "installed": installed, "why": why, "transcribe_available": transcribe_available,
        })
        .to_string());
    }
    #[cfg(not(agent_shell))]
    if cmd == "agent_status" || cmd == "set_agent" || cmd == "set_transcribe" {
        // A notes-only build: same shape again, and `why` says which kind of build this is rather
        // than leaving the row blank.
        return Ok(serde_json::json!({
            "enabled": false, "transcribe": false, "ok": true,
            "installed": false,
            "why": "This build of the app was made without the study assistant. Everything else in \
                    formicaria works normally.",
            "transcribe_available": false,
        })
        .to_string());
    }
    // **Updating the app.** A transport concern, like the agent's rows above: fm-serve answers these on
    // the desktop and the core `dispatch` never has them. **Matched by exact name** — `update_body` is
    // a note being saved, and a prefix match would swallow every edit made on this phone.
    #[cfg(update_shell)]
    if matches!(
        cmd.as_str(),
        "update_status"
            | "update_check"
            | "set_update_check"
            | "update_start"
            | "update_cancel"
            | "update_apply"
            | "update_rollback"
    ) {
        use tauri::Manager;
        let cache = app_handle.path().app_cache_dir().map_err(|e| e.to_string())?;
        if let Some(r) = update::handle(&cmd, &args, &cache) {
            return r;
        }
    }
    // A build that cannot update itself — iOS, or one made without the feature — still answers the
    // status the panel reads, so the rows hide instead of the panel meeting an unknown command.
    #[cfg(not(update_shell))]
    if cmd == "update_status" {
        return Ok(serde_json::json!({
            "can_check": false, "can_install": false,
            "why": "This build of the app cannot update itself.",
            "current": null, "available": null, "previous": null, "can_go_back": false,
            "checking": false, "check": false, "last_check": 0, "error": null, "progress": null,
            "page": null,
        })
        .to_string());
    }
    // Resolved here rather than in the signature, so the arms above answer while the vaults are
    // still opening (the `@`-picker and the Settings toggles need no store) and this one reports
    // *why* it cannot.
    let app = vault_state(&app_handle)?;
    // **Logged before it is returned.** The UI shows the message, but a phone screen is not
    // somewhere a stack of failures can be compared — and the whole point of the tag is that
    // a failing clone can be read off `adb logcat` instead of retyped by hand.
    let out = dispatch(&cmd, &args, &[], &app, &MobileHost).inspect_err(|e| {
        log::error!("{cmd}: {e}");
    })?;
    String::from_utf8(out.into_bytes()).map_err(|e| e.to_string())
}

/// Where this device keeps our data.
///
/// **This is the hole Settings found.** `fm_app::vaults::config_dir()` falls to a catch-all arm
/// on Android that reads `FM_CONFIG_DIR`, and nothing sets it — so without this a phone cannot
/// persist a vault list at all. Tauri knows the platform's app-data directory, so the shell is
/// the right place to supply it: it is exactly the kind of fact only the platform has, which is
/// why it is set here rather than guessed inside `fm-app`.
fn configure_paths(handle: &tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    // **A silent early return produced a *wrong* screen, not a blank one** (fixed 2026-09-04).
    //
    // This was `let Ok(dir) = … else { return };`. With nothing set, `config_dir()` answers `None`,
    // `App::load` succeeds with **zero vaults**, and the user meets the ordinary first-run form —
    // whose `create_vault` then cannot persist anything, because there is nowhere to write the
    // vault list. Indistinguishable from a fresh install, and every attempt to fix it by hand
    // fails the same way.
    //
    // `install_logger()` runs immediately before this call, and its own doc says that ordering
    // exists *"precisely so that everything after it — including its own failures — is visible"*.
    // The infrastructure was there; nothing used it.
    let dir = match handle.path().app_data_dir() {
        Ok(d) => d,
        Err(e) => {
            // **The environment may already have answered.** A desktop debug run of this library
            // sets `FM_CONFIG_DIR` itself, and there `app_data_dir()` failing is not fatal — so
            // the refusal is scoped to the case where nothing else has supplied a location.
            if std::env::var_os("FM_CONFIG_DIR").is_some() {
                log::warn!("no platform data directory ({e}) — using FM_CONFIG_DIR from the environment");
                return Ok(());
            }
            let msg = format!(
                "this device gave the app no data directory ({e}), so there is nowhere to keep a \
                 vault. Notes cannot be created or opened until that is resolved."
            );
            log::error!("{msg}");
            return Err(msg);
        }
    };
    let _ = std::fs::create_dir_all(&dir);
    // Safety: single-threaded, before any vault is opened. `set_var` is the only way to reach
    // `config_dir()`, which reads the environment by design so that the same code works under
    // `fm-serve`, the CLI and here.
    // Every vault this device makes lives in here, and nothing else on the device writes to it.
    //
    // **A phone has no place a user could name.** There is no `$HOME`, no shell to `mkdir`
    // with, and no file manager that can reach an app's storage — so the desktop's "type a
    // folder" question has no answer here, and asking it produced a literal `~/notes` resolved
    // against the process working directory. The platform gives an app exactly one directory it
    // may write to; this is it, and `fm-app` puts vaults inside it by name.
    //
    // **The cost, stated:** Android deletes this when the app is uninstalled. That is the price
    // of a sandbox nothing else can touch, and it is the reason a phone vault wants a git remote
    // or a restic repo pointed at it rather than being the only copy.
    let root = dir.join("vaults");
    let _ = std::fs::create_dir_all(&root);

    // Safety: single-threaded, before any vault is opened. `set_var` is the only way to reach
    // `config_dir()`, which reads the environment by design so that the same code works under
    // `fm-serve`, the CLI and here.
    unsafe {
        std::env::set_var("FM_CONFIG_DIR", &dir);
        std::env::set_var("FM_VAULT_ROOT", &root);
        // First run needs *a* vault or the app opens on the first-run screen with nowhere to
        // create one — on a phone there is no shell to `mkdir` with.
        if std::env::var_os("FM_VAULT").is_none() {
            // **The pre-root default is honoured when it exists.** Earlier builds put the first
            // vault at `<app_data>/vault`, outside the root introduced here. Repointing at the
            // new location would leave those notes on disk and invisible — indistinguishable
            // from data loss to the person holding the phone. So an existing one keeps its
            // path, and only a fresh install starts inside the root.
            let legacy = dir.join("vault");
            let vault = if legacy.exists() { legacy } else { root.join("notes") };
            let _ = std::fs::create_dir_all(vault.join("notes"));
            std::env::set_var("FM_VAULT", &vault);
        }
    }
    // Load-bearing, like the `vaults ready` line in `boot`: `ci/android-smoke.sh` asserts this
    // appears **before** it, which is what proves the paths were configured rather than skipped.
    log::info!("vault root: {}", root.display());
    Ok(())
}

/// Whether the paths were configured — set once, in `setup`, and read by every later `boot`.
///
/// **A log line alone would not have been enough.** Without this, `boot` still succeeds with zero
/// vaults, `vault_state` clears its `last` error, and the message evaporates behind a first-run
/// form the user cannot complete. The verdict has to outlive the log.
///
/// Only the *paths* verdict is once-only. Opening the vaults stays retryable, which is the whole
/// reason `boot` is callable more than once.
static PATHS: std::sync::OnceLock<Result<(), String>> = std::sync::OnceLock::new();

/// Give the vendored OpenSSL a CA trust store, without which **every** HTTPS remote fails.
///
/// `git2` links OpenSSL statically, and a vendored build bakes in a default certificate
/// directory that does not exist on Android. With no trust store libgit2 reports "the SSL
/// certificate is invalid" for a perfectly good github.com — an error that names the server
/// rather than the missing bundle, which is exactly how it wasted an evening.
///
/// **Not `SSL_CERT_DIR` pointed at Android's store**, which looks correct and silently does
/// nothing: that lookup is by hashed filename, and Android names its certificates with
/// OpenSSL's *pre-1.0.0* subject hash while a modern OpenSSL computes a different one. Measured
/// on the device — `01419da9.0` on disk, `8d89cda1` from `openssl -subject_hash`. See
/// [`fm_app::ca_bundle`] for the full reasoning; it concatenates instead, which uses no hashed
/// lookup at all.
///
/// Both directories are offered because which exists varies by version: the historical
/// `/system/etc/security/cacerts` and the Conscrypt APEX that newer releases serve from. On the
/// measured device both are present and overlap, which the bundle deduplicates.
///
/// Best-effort and non-fatal: a phone that cannot build a bundle is still a working notebook —
/// it just cannot reach an HTTPS remote, which is the same position it was in before.
#[cfg(target_os = "android")]
fn install_ca_bundle(handle: &tauri::AppHandle) {
    use tauri::Manager;
    let Ok(dir) = handle.path().app_data_dir() else { return };
    let dirs = [
        std::path::Path::new("/apex/com.android.conscrypt/cacerts"),
        std::path::Path::new("/system/etc/security/cacerts"),
    ];
    match fm_app::ca_bundle::install(&dirs, &dir.join("ca-bundle.pem")) {
        Ok(n) => log::info!("ca-bundle: {n} certificates loaded into libgit2"),
        // Said out loud rather than swallowed: this is the difference between "sync is broken"
        // and "sync cannot verify anyone", and the two look identical from the UI.
        Err(e) => log::error!("ca-bundle: {e}"),
    }
}

/// Send this shell's diagnostics somewhere they can actually be read.
///
/// **Neither mobile platform routes Rust's stdout or stderr anywhere by default.** Every
/// `eprintln!` in this file has been writing into a void — which is precisely how "the SSL
/// certificate is invalid" stayed unexplained across several builds while the app was already
/// reporting the cause. A startup diagnostic nobody can read is not a diagnostic.
///
/// - **Android** — everything lands under the `formicaria` tag: `adb logcat -s formicaria`.
/// - **iOS** — everything lands on stderr, which `xcrun simctl launch --console-pty` attaches to a
///   pty and prints. That is the only consumer there will ever be: the owner has no Mac and no
///   iPhone, so a Simulator under CI is the sole place this shell can run (see
///   `docs/context/ios-plan-2026-09-02.md`).
///
/// **iOS deliberately does not use `os_log`**, though the plan first named it. Emitting to it
/// needs `_os_log_impl` with a format descriptor placed in `__TEXT,__os_log` — in practice a C
/// shim and a new crate — and neither could be *compiled*, let alone read back, from this machine.
/// Getting that section wrong does not fail to build; it logs `<private>`, which is a diagnostic
/// channel that lies. Reaching for an untestable dependency to serve a reader that the testable
/// channel already serves is the worse trade. Recorded under `#track-m` in `decisions.md`.
fn install_logger() {
    #[cfg(target_os = "android")]
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("formicaria"),
    );

    // `set_logger` may only succeed once per process. `install_logger` is called once, from
    // `setup` — but a shell whose entire job is to stay up so it can report a failure must not
    // panic on a second call, so the failure is a no-op rather than `unwrap`.
    //
    // **A `OnceLock` static rather than `set_boxed_logger`**, which needs an owned `Box` and lives
    // behind `log`'s `std` feature — not enabled here, so it does not compile. Caught before it
    // reached a runner by building this file's logger in a throwaway crate on Linux; changing a
    // shared dependency's features to reach a convenience function would have been the worse fix.
    #[cfg(target_os = "ios")]
    if log::set_logger(IOS_LOGGER.get_or_init(IosLogger::new)).is_ok() {
        log::set_max_level(log::LevelFilter::Info);
    }
}

/// Storage for the one logger, so `set_logger` gets the `&'static` it requires without a `Box`.
#[cfg(target_os = "ios")]
static IOS_LOGGER: std::sync::OnceLock<IosLogger> = std::sync::OnceLock::new();

/// The iOS backend for [`install_logger`]: every record to **a file inside the app container**,
/// and to stderr as well.
///
/// **stderr alone was measured to reach nobody, and that is why the file exists.** The first
/// version wrote only to stderr, on the reasoning that `xcrun simctl launch --console-pty` attaches
/// a pty and prints it. Run 91364602829 disproved that: the app demonstrably ran — the unified log
/// carries thirty seconds of its WebKit traffic, resources loading through Tauri's scheme handler —
/// while the pty capture was **byte-empty** and the unified log contained **not one** of our
/// records. So an iOS build had no voice at all, and a startup failure would have been invisible in
/// exactly the way `install_logger` exists to prevent.
///
/// A file is the one channel that cannot be argued with: no bridging assumption, no FFI, no C shim,
/// no framework to link. `std::env::temp_dir()` on iOS is the app's own `tmp/`, inside the data
/// container, which `xcrun simctl get_app_container` hands the smoke test directly. Simulator-only
/// reasoning, and allowed to be — a Simulator is the only place this shell runs (`decisions.md`).
///
/// stderr is kept because it costs one line and would start working for free if a future
/// `simctl`/Xcode fixed the pty. A logger with two sinks and no dependencies is cheaper than
/// deciding which one to trust.
#[cfg(target_os = "ios")]
struct IosLogger {
    /// `None` if the file could not be opened. **Never a reason to fail**: a shell that cannot
    /// write its log must still start, and stderr may yet be read by something.
    file: std::sync::Mutex<Option<std::fs::File>>,
}

#[cfg(target_os = "ios")]
impl IosLogger {
    /// The path is deliberately *not* `app_data_dir()`: `install_logger` runs before
    /// `configure_paths`, precisely so that everything after it — including its own failures — is
    /// visible. `temp_dir()` needs no Tauri handle and is inside the container either way.
    fn new() -> Self {
        let path = std::env::temp_dir().join("formicaria.log");
        let file = std::fs::OpenOptions::new().create(true).append(true).open(&path).ok();
        Self { file: std::sync::Mutex::new(file) }
    }
}

#[cfg(target_os = "ios")]
impl log::Log for IosLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        // The `formicaria` prefix mirrors Android's logcat tag on purpose: one grep, and the two
        // smoke tests assert the same startup lines rather than drifting apart.
        let line = format!(
            "formicaria {:<5} {}: {}",
            record.level(),
            record.target(),
            record.args()
        );
        eprintln!("{line}");
        // Flushed per record rather than buffered: a diagnostic still in a buffer when the process
        // dies is not a diagnostic, and this file exists for exactly the launches that end badly.
        // A poisoned lock or a write error is swallowed — logging must never be why the app fell over.
        if let Ok(mut slot) = self.file.lock() {
            if let Some(f) = slot.as_mut() {
                use std::io::Write;
                let _ = writeln!(f, "{line}");
                let _ = f.flush();
            }
        }
    }

    /// Nothing to push: both sinks write straight through.
    fn flush(&self) {}
}

/// Forwarded from the WebView by [`WEB_LOG_SCRIPT`]. **A transport concern, not a notes command** —
/// the same category as `fm_ingest`, so `fm_app::dispatch` remains the only door for anything about
/// notes (`decisions.md#track-m`, *the WebView gets a voice*).
///
/// `(async)` like every other command here, and for the reason `ci/checks.sh` enforces: Android
/// never gets Tauri's async custom-protocol IPC, so a blocking command freezes the screen for its
/// duration. A logging call that froze the UI would be a diagnostic that causes the symptom.
///
/// The level is clamped here rather than trusted: it arrives from the page.
#[tauri::command(async)]
fn fm_log(level: String, msg: String) {
    // Truncation is the shell's job, not the script's — a page that stopped cooperating is exactly
    // the case this exists to report, so the guard has to be on this side of the boundary too.
    let msg: String = msg.chars().take(2000).collect();
    match level.as_str() {
        "warn" => log::warn!("web: {msg}"),
        _ => log::error!("web: {msg}"),
    }
}

/// Injected into every page, on both platforms, before the frontend runs.
///
/// **It may never break the app**, which is what most of its length is about: the whole body is
/// inside `try`/`catch`, the original `console` method is always called first so a dead bridge
/// leaves today's behaviour exactly as it is, a re-entrancy flag stops a failure in the forwarding
/// path recursing through the `console.error` it just overrode, and messages are buffered — with a
/// hard cap, dropped rather than grown — while `__TAURI_INTERNALS__.invoke` does not yet exist.
///
/// `console.log` is deliberately not forwarded. Only failures: a phone log carrying every debug
/// line is a phone log nobody reads.
const WEB_LOG_SCRIPT: &str = r#"
(function () {
  try {
    var MAX = 2000, QUEUE_MAX = 50, queue = [], busy = false;
    function send(level, text) {
      if (busy) return;                       // never recurse through our own console.error
      busy = true;
      try {
        var body = String(text).slice(0, MAX);
        var inv = window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke;
        if (inv) {
          while (queue.length) { var q = queue.shift(); inv('fm_log', q); }
          inv('fm_log', { level: level, msg: body });
        } else if (queue.length < QUEUE_MAX) {
          queue.push({ level: level, msg: body });  // bridge not up yet; bounded, never grown
        }
      } catch (e) { /* a diagnostic must not become the fault */ }
      busy = false;
    }
    function text(args) {
      return Array.prototype.map.call(args, function (a) {
        try {
          if (a instanceof Error) return (a.stack || (a.name + ': ' + a.message));
          return typeof a === 'string' ? a : JSON.stringify(a);
        } catch (e) { return String(a); }
      }).join(' ');
    }
    ['error', 'warn'].forEach(function (level) {
      var original = console[level] ? console[level].bind(console) : function () {};
      console[level] = function () {
        original.apply(null, arguments);      // first, always: the bridge is additive
        send(level, text(arguments));
      };
    });
    // **Two different events wear the same name, and conflating them was the first version's bug.**
    // A script error targets `window` and carries `message`/`filename`. A *resource* failure — an
    // <img>, a <script>, a stylesheet — targets the element, carries no message, and would have
    // been reported as the contentless string "error": a red job with nothing to act on, and a
    // false positive for any optional asset. Named separately, a failed `fmblob:` image reports the
    // URL that failed, which is the single most useful line this bridge can produce on this app.
    window.addEventListener('error', function (e) {
      var t = e && e.target;
      if (t && t !== window && (t.src || t.href)) {
        send('error', 'resource failed to load: ' + (t.src || t.href));
        return;
      }
      send('error', (e && e.message ? e.message : 'error') +
        (e && e.filename ? ' @ ' + e.filename + ':' + e.lineno : ''));
    }, true); // capture: resource errors do not bubble
    window.addEventListener('unhandledrejection', function (e) {
      var r = e && e.reason;
      send('error', 'unhandled rejection: ' + (r && r.stack ? r.stack : String(r)));
    });
  } catch (e) { /* an init script that throws is a blank screen */ }
})();
"#;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // **Before `.setup`, and that ordering is the point.** Tauri builds the webview before the
        // setup hook runs — the comment in that hook says so — so a script registered later would
        // miss the page load it most needs to observe. A plugin is how Tauri v2 injects one
        // globally; there is no per-app `initialization_script` for config-declared windows.
        .plugin(
            // `::<Wry, ()>` explicitly: `plugin::Builder` is generic over the runtime *and* a
            // config type it will never deserialise here, and neither is inferable from a builder
            // that only sets a script.
            tauri::plugin::Builder::<tauri::Wry, ()>::new("weblog")
                .js_init_script(WEB_LOG_SCRIPT.to_string())
                .build(),
        )
        .setup(|app| {
            // First, so that everything below is visible — including its own failures.
            install_logger();
            // **Recorded, not just logged.** See `PATHS`: a device that gave us nowhere to write
            // must not present a first-run form whose every button fails.
            let _ = PATHS.set(configure_paths(app.handle()));
            // Clear an update that has already been installed, then look for a newer one in the
            // background — after `configure_paths`, because the settings file lives under the
            // directory it sets.
            #[cfg(update_shell)]
            {
                use tauri::Manager;
                update::on_start(app.handle().path().app_cache_dir().ok());
            }
            // The git token, if this device has one. **Only ever reached here**: a desktop
            // delegates to git's credential helper and stores nothing, so this call is the
            // mobile half of that split (`fm_app::secrets`). Must run before the first sync,
            // and after `configure_paths` — it reads from the config directory that sets up.
            // Cheap (one small file), so it stays on the critical path.
            fm_app::secrets::install_into_env();
            // Opening the vaults is the one slow thing left before the app can answer anything.
            //
            // **Nothing slow may be added above this line, and a failure here must not panic.**
            // The webview is built by Tauri *before* this hook runs (`tauri::app::setup` creates
            // the config windows first), so the page is already loading and already invoking
            // while we are here — and the event loop that delivers a reply does not start until
            // this returns. Whatever goes wrong, the phone must end up with a backend that can
            // say so: a `?` here fails the hook, which panics this thread and leaves the Activity
            // holding a webview with nothing behind it. That is not a crash the user can see or
            // report — it is a screen that never paints. Diagnosed on the owner's phone
            // 2026-07-31 ("gray screen on the first open, fine on the second").
            //
            // So the whole of it lives in `boot`, which cannot fail upward, and which the first
            // arriving command will attempt again if this pass did not manage it.
            if let Ok(mut last) = BOOT.lock() {
                *last = boot(app.handle()).err();
            }
            Ok(())
        })
        .register_uri_scheme_protocol("fmblob", |ctx, req| {
            blob_response(
                ctx.app_handle(),
                req.method().as_str(),
                &req.uri().to_string(),
                req.body(),
                // **Forwarded, because the handler cannot answer a seek without it.** Until
                // 2026-09-04 this was not passed at all and the handler had no way to know.
                req.headers()
                    .get("Range")
                    .and_then(|v| v.to_str().ok())
                    .map(str::to_string),
            )
        })
        .invoke_handler(tauri::generate_handler![
            fm,
            fm_ingest,
            fm_ingest_chunk,
            fm_ingest_finish,
            fm_ingest_cancel,
            fm_log
        ])
        .build(tauri::generate_context!())
        .expect("error while running formicaria")
        .run(|_app, event| {
            // When the app exits, stop the study agent's model — trip its off-switch so the
            // llama-server child is killed rather than left to be reaped (PDEATHSIG is the backstop).
            #[cfg(agent_shell)]
            if let tauri::RunEvent::Exit = event {
                agent::stop();
            }
            let _ = &event;
        });
}
