# 2026-07-31 — the gray screen on the first open

**The report.** *"Every time I try to open [the Android app] I get a gray screen. Only after closing
it and opening it again it starts."* The owner's phone, the 2026-07-24 release APK. Confirmed by them
as happening on a genuinely cold start too, with the fix being a swipe-away from Recents.

## What the phone said before anything was changed

Read-only probes over wireless `adb` (no installs, no writes — `mobile-design.md` § "Device safety"):

- The app process had been alive **3 hours** in the background, holding two children:
  `libllama-server.so` (LFM2.5-1.2B, :8081) and `libwhisper-server.so` (ggml-base.en, :8082), both
  almost entirely **swapped out** — RSS ~2.5 MB against a ~12.7 GB VSZ.
- 7.6 GB RAM, ~2.4 GB available, **8 GB swap with ~3 GB in use**, and `lowmemorykiller` actively
  reaping **Chrome, the Play Store and Facebook** while formicaria survived — because its foreground
  service outranks them.
- **~1 SELinux denial per second from our own process**: `avc: denied { read } … name="loadavg"`.
  The log buffer was mostly ours, and mostly that.
- The app's own PSS was 83 MB of which **59 MB was `SwapPss`** — the app itself had been paged out.

## The mechanism (read out of the code, not guessed)

1. **Tauri creates the webview *before* the `setup` hook runs** —
   `tauri-2.11.5/src/app.rs:2521-2535` builds the config windows first. So the page is loading and
   already invoking while the shell is still starting.
2. **tao runs the hook on a spawned thread**, not the Android UI thread
   (`tao-0.35.3/src/platform_impl/android/ndk_glue.rs:323-380`) — so this was never an ANR.
3. The hook did, in order: `install_ca_bundle` (the whole system cert store) → `secrets` →
   `App::load()` (**a full FTS reindex of every vault**) → agent start → and only then
   `Manager::manage`. **The event loop that delivers an IPC reply does not start until the hook
   returns.**
4. `App.svelte`'s render gate rendered **nothing** while `vaults === null`. And the failure path
   `.catch(() => (vaults = []))` rendered the **first-run "create your first vault" form**.

So: an unanswered `list_vaults` was a permanently blank screen, and a refused one was a lie. A
failure inside the hook was worse than either — `?` fails the hook, which **panics that thread** and
leaves the Activity holding a live webview with no backend behind it. Nothing crashes. Nothing logs
anywhere the owner can read (Rust's stdout is not routed to logcat, the WebView forwards no
`console.*`, and MIUI suppresses our tag). It is an unreportable blank window.

**Not the cause, though it looked like one:** the WebView renderer being reclaimed. `RustWebViewClient`
has no `onRenderProcessGone` override and the framework default *kills the app process*, so that
presents as the app vanishing, not as a gray screen.

## What changed

**The shell** (`mobile/src-tauri/src/lib.rs`) — startup is now one function, `boot`, which cannot
fail upward: `catch_unwind` around the open, the store `manage`d the instant it exists, and the cert
store + model start moved *after* it onto one thread. A failure is recorded in `BOOT` (a `Mutex`,
**not** a `OnceLock` — the first draft froze the message for the life of the process, so the UI's
retry button provably could not help) and every command answers with it. `boot` is re-attempted by
the first command that finds no store, so "Try again" genuinely retries.

**The gate** (`ui/src/App.svelte`, `ui/src/lib/Starting.svelte`) — `vaults === null` renders a
startup screen: silent for 700 ms so a normal launch never flashes it, then "Opening your vaults…",
then at 8 s an escalated wording plus the backend's verbatim reason and a retry. The boot poll is
**time-driven, not rejection-driven**: the symptom was a call that never settled, and a `.catch` never
sees one. `list_views` is keyed on `vaults` so one early refusal no longer costs the user their saved
views for the session.

**`main.ts`** — the theme read is guarded and a mount failure now writes words into the page. It is
plain DOM in the module, not an inline script in `index.html`: the mobile CSP has no `'unsafe-inline'`,
so an inline fallback would itself fail silently.

**Closure** — `pagehide`/`visibilitychange` flush the pending 500 ms save and the pending 5 s
auto-commit. On Android **Back is a kill** (tao calls `process::exit` when the last window closes) and
swipe-away/LMKD are `SIGKILL`, so "in five seconds" often never arrives. Best-effort only: a WebView
is not guaranteed to deliver those events before teardown.

**The agent** — `Control` gained a **generation counter**. Turning the assistant off during the first
~1.4 GB model download used to be silently lost (the stopper does not exist yet), leaving `running =
false` with a model in RAM, which let the next on-toggle spawn a *second* `llama-server` on the same
port. And `AgentService` is now `START_NOT_STICKY`: sticky restarted the process **for the service
only** — no Activity, so no Rust, no vault, no model — leaving a permanent "Running on your device"
notification over an empty JVM holding LMKD priority.

**The watchdog** — `/proc/loadavg` is no longer *attempted* on Android. Tolerating the denial was not
enough: the kernel audits every denied open, so a tolerated read on a 2 s timer wrote a line per
second into the one diagnostic channel a phone has.

## The tests, which are the actual deliverable

The bug was invisible to `pixi run ci` for one reason: **`mock.ts` always succeeded, immediately**,
and nothing ever starts the mobile shell.

- `ui/src/lib/mock.ts` gained a fault surface — `faults([{cmd, mode: 'reject'|'hang'|'delay'}])`.
- `ui/src/App.boot.test.ts` drives the app against a backend that refuses, stalls and **never
  answers**: no gate state may render an empty document, a refusal shows its own words and never the
  first-run form, a hang escalates and self-recovers. **4 of these 5 fail against the pre-fix gate**
  (verified by reverting it).
- `ui/src/lib/Starting.svelte.test.ts` — silent at first, speaks after.
- `ci/checks.sh` gained two structural guards, since the mobile crate cannot compile in this env at
  all: no `?`/`unwrap`/`expect`/`panic` in the setup hook or in `boot`; the store managed before
  `install_ca_bundle`/`agent::start`; `BOOT` present and not a `OnceLock`; the `Starting` branch and
  the boot test file still there. **Both verified to fail on a deliberately reintroduced regression.**
- **`pixi run android-smoke`** (`ci/android-smoke.sh`) — the unattended emulator test. Two
  consecutive cold launches must **both** paint (the bug's whole signature was *fine the second
  time*, so a one-launch test would have stayed green through the outage); "painted" is
  `vips deviate <screencap> > 10`, with every measured number written to `stats.txt`; the
  `vaults ready` line must precede every `ca-bundle`/`study agent` line; a resume must **not** re-run
  the hook; Back must leave no model process behind; and a relaunch after an OS background-kill must
  paint. **Emulator only** — it installs and force-stops, so it refuses any serial that is not
  `emulator-*`. That refusal was verified with the owner's phone attached.

## Still open (found by the audits, not fixed here)

Recorded in `known-issues.md`: the phone's `fmblob` handler buffers whole blobs (ruling 7's `Range`
streaming was never built there); the model fetcher verifies neither length nor hash; `activity` holds
the vault mutex across a per-vault revwalk on the first refresh; a poisoned vault mutex bricks every
command for the life of the process.

*(Two items that were listed here were fixed later the same day — `MultiStore::open`'s fail-fast open
and the missing `cleanup_state`/`repo.state()` handling in `commit_all`. See the second half of this
file.)*

## Lessons

- **A screen that renders nothing is not a neutral default; on a phone it is a bug with no bug
  report.** The gate's "unknown is not no" instinct was right and its implementation was wrong: the
  answer is a screen that says "unknown", not the absence of one.
- **A mock that only ever succeeds is a mock that hides every failure path it has.** This was already
  written down in `known-issues.md` about one hand-written guard. It took a phone to generalise it.
- **The same first-paint contention bug had already been fixed once**, on the desktop, one week
  earlier (`decisions.md`, "the study agent's model warm-up is deferred"). Nobody asked whether the
  phone had the same shape. It did, worse — because there the model load and the *vault open* were
  both in front of the first frame.

---

## Later the same day: the conflict nobody could resolve, and the notes nobody knew were missing

The owner reported *"I have a conflict on the meeting template but I see nothing, no option to solve
it"*. Diagnosed on the real vault: a `DU` (deleted-here, edited-there) conflict on the Meeting
template had left the development vault **mid-merge for 7 days**, with **95 notes never
committed**, because `commit_all` refuses while a vault is mid-merge. Resolved by accepting the
incoming version, committing the backlog and pushing (local and `origin/main` both `e44751d`).

Then four defects, in the order they were found — each one uncovered by fixing the last:

1. **Two definitions of "conflict".** `commit` asked git; the UI's list scanned note bodies for
   `<<<<<<<`. A delete/modify has no markers, so the app warned about a conflict it could not show,
   and its advice ("open each one, both versions are marked in the text") was impossible to follow.
   → `vcs::conflicted` carries the kind; `resolve_conflict` keeps a side; the Collaboration surface
   offers both, and the row says in plain words what the two sides did.
2. **`commit_all` could permanently skip a note.** It stages only paths `FileStore::put`/`delete`
   recorded, and that memory dies with the process — so every note written before the last restart
   was unstageable *forever*, silently. → `vcs::unrecorded` + a toolbar chip + an explicit
   `record_unrecorded`. It found a real one on the first run against the live vaults.
3. **Editing a note resolves nothing as far as git is concerned.** Verified against real git: clean
   text over a `UU` path leaves all three index stages. So the documented resolution settled nothing,
   and because the list was marker-derived, the note *left the UI* as the markers were tidied —
   taking the only sign of trouble with it. → `Keep::Edited` ("Mark resolved"), refused while markers
   remain, sharing one `merge::has_conflict_markers` with the list.
4. **Both backends' `commit_all` were broken on the merge path, in opposite directions** — found by
   the differential test written for (3), which failed on its first run exactly as intended. The
   phone silently committed conflict markers as note content and dropped the merge's second parent;
   the laptop errored with *"cannot do a partial commit during a merge"*. → both refuse while
   unmerged, both commit a merge with two parents and `cleanup_state()`.

Also: `MultiStore::open` no longer refuses every vault when one will not open — it opens the rest and
**names** the failures on the heartbeat, because a vault silently absent reads as lost notes.

**Lessons.**
- **Derive a surface from the authority, not from a symptom.** Markers are the symptom of one of
  git's seven conflict codes. Everything without that symptom was invisible.
- **A compiler that checks signatures says nothing about behaviour.** `vcs.rs` claims the two
  backends "cannot drift in shape without the compiler saying so" — true, and both had the same
  function with opposite bugs. `ci/checks.sh` now greps shape parity; behaviour is
  `pixi run test-native-git`, which is opt-in, so run it when either backend changes.
- **"History can lag" and "history can be skipped" are different claims.** The second needs a count
  on screen, because nobody audits git by hand.
- **Write the differential test first.** Both of (4)'s bugs were found in the first run, before any
  fix existed — which is the entire argument for grading a harness before trusting it.
