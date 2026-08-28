#!/bin/sh
# Architectural guards, enforced in CI. Any hit fails the build. These are not
# style checks — they defend the two invariants the whole design rests on:
# (1) the query engine never touches storage, and (2) renderers are generic.
# Run locally with:  pixi run checks

fail=0

echo "[check] seam: fm-query must not depend on a database crate..."
if cargo tree -p fm-query 2>/dev/null | grep -Eiq 'rusqlite|libsqlite3|sqlx|diesel'; then
    echo "  FAIL: fm-query pulls in a storage crate — the query engine must stay storage-free."
    fail=1
fi

echo "[check] seam: fm-query source must not reference the filesystem..."
# Match code, not the doc comments that merely mention these names.
#
# The filter drops lines whose CONTENT begins with `//` — i.e. comment-only lines — by
# anchoring past grep's own `path:lineno:` prefix. It used to be a bare `grep -v '//'`, which
# dropped any line containing `//` anywhere, so `use std::fs; // temporary` sailed straight
# through the guard. Verified: with the old filter that line is MISSED, with this one it is
# CAUGHT. A guard that is disarmed by adding a comment is worse than no guard, because it
# reads as protection.
if grep -REn 'std::(fs|path)|std::io::[A-Za-z]*File' crates/fm-query/src \
    | grep -vE '^[^:]+:[0-9]+:[[:space:]]*//'; then
    echo "  FAIL: fm-query references a filesystem API — the seam is broken."
    fail=1
fi

echo "[check] seam: the notes-only base filter has exactly one definition..."
# `decisions.md`, from the assets exclusion that came first: "Filtering in each renderer was
# rejected — it must be repeated per view and silently forgotten by the next one." That
# prediction came true twice: the discussion ruling enumerated three surfaces, and by the time
# anyone built it `.view` presets and `activity` had shipped in between. So the base filter
# lives in `thread::notes_base()` and nowhere else, and this check is what keeps it there.
#
# `activity` is deliberately exempt: it is a `git log` read-model with no `Filter` to hang a
# predicate on, so it uses `thread::is_message` / `thread::is_proposal` instead. That is why the
# exemption is a *file* and not a blanket allowance.
if grep -REn 'Predicate::Kind\(vec!\[Kind::Note\]\)' crates/fm-app/src \
    | grep -v '^crates/fm-app/src/thread.rs:'; then
    echo "  FAIL: build the notes-only filter with thread::notes_base(), not by hand —"
    echo "        a hand-rolled Kind(Note) omits the message and proposal exclusions."
    fail=1
fi

echo "[check] the app reaches git only through fm_core::vcs..."
# vcs.rs picks the backend; git.rs shells out and there is no `git` binary on Android. Naming a
# backend at a call site is what left the phone reporting "git not installed" while carrying a
# working libgit2 — twice: once for history, and again for the whole proposal lifecycle, which a
# unit test could never catch because the callers simply never called.
#
# Shared TYPES (Accepted/Identity/Pulled/Touch/Probe/HelperAdvice) are UpperCamel, so the [a-z_]
# match excludes them — casing does the allowlisting. The named exceptions genuinely ask "is there
# a git BINARY": credential helpers are a CLI concept libgit2 has no equivalent for. Tests are out
# of the search path by design — the differential harness *must* name both backends.
if grep -REn 'fm_core::git::[a-z_]+|use fm_core::git;' crates/*/src mobile/src-tauri/src \
    | grep -v '^crates/fm-core/src/' \
    | grep -vE '^[^:]+:[0-9]+:[[:space:]]*//' \
    | grep -vE 'fm_core::git::(available|credential_approve|credential_exists|credential_helper|helper_advice)\b'; then
    echo "  FAIL: call fm_core::vcs::… — fm_core::git shells out, and a phone has no git binary."
    fail=1
fi

echo "[check] every routed vcs operation exists on both git backends..."
# `vcs.rs` routes each operation to one of two backends. Its own comment says the `#[cfg]` lives
# *inside* each function "so that a build without the feature still compiles every signature
# identically — the two backends cannot drift in shape without the compiler saying so". True, and only
# for a build that HAS the feature: `pixi run ci` does not build `native-git` (it would compile libgit2
# + OpenSSL from source), so a function added to `git.rs` alone sails through this gate and breaks only
# the phone — which is the device nobody can debug. This grep is that missing half.
#
# **Shape only.** Behavioural parity is what `pixi run test-native-git` is for, and on 2026-07-31 it
# earned that distinction: both backends had `commit_all`, with opposite bugs — the subprocess one
# erroring on every merge commit, the libgit2 one silently committing conflict markers as content.
# Do not read a green grep here as "the backends agree".
missing_backend=""
for fn in $(grep -oE '^route!\([a-z_]+' crates/fm-core/src/vcs.rs | sed 's/^route!(//'); do
    grep -qE "^pub fn ${fn}\b|^pub fn ${fn}\(" crates/fm-core/src/git.rs \
        || missing_backend="$missing_backend git.rs:$fn"
    grep -qE "^pub fn ${fn}\b|^pub fn ${fn}\(" crates/fm-core/src/git_native.rs \
        || missing_backend="$missing_backend git_native.rs:$fn"
done
if [ -n "$missing_backend" ]; then
    echo "  FAIL: routed through vcs but missing from a backend:$missing_backend"
    echo "        A routed operation must exist in BOTH git.rs and git_native.rs. Missing it in"
    echo "        git_native.rs compiles fine here (ci does not build native-git) and breaks only the"
    echo "        phone, where there is no git binary and no way to read the error."
    fail=1
fi

echo "[check] renderers must not hardcode status values..."
if [ -d ui/src/renderers ]; then
    if grep -REniw 'todo|doing|done' ui/src/renderers; then
        echo "  FAIL: a renderer hardcodes a status value; group-by must be generic."
        fail=1
    fi
else
    echo "  (skip: ui/src/renderers does not exist yet)"
fi

echo "[check] no unaudited HTML sink in the UI (note bodies are untrusted in a shared vault)..."
# A note body arrives from collaborators through the `.md` merge driver, so any `innerHTML`/
# `{@html}` is a stored-XSS sink. There are exactly three legitimate ones, all in render.ts and all
# guarded (DOMPurify / KaTeX throwOnError / Mermaid securityLevel:'strict'); each is tagged
# `// sink-ok:`. A new, untagged sink fails the build — forcing a conscious review, not a silent
# reopening. Same comment-anchor trick as the fm-query guard above.
# Match only *writes* — `.innerHTML =`/`.outerHTML =` (assignment, not the `==` of a read
# comparison), `insertAdjacentHTML(`, and Svelte `{@html`. Reads (`x = el.innerHTML`, a test
# assertion) are not sinks. Test files are excluded — they do not ship.
if grep -REn '\.(inner|outer)HTML[[:space:]]*=[^=]|insertAdjacentHTML|\{@html' ui/src \
    | grep -vE '\.test\.ts:' \
    | grep -vE 'sink-ok'; then
    echo "  FAIL: unaudited HTML sink. After review, tag the line '// sink-ok: <reason>', or remove it."
    fail=1
fi

echo "[check] Mermaid must never run with securityLevel 'loose'..."
# 'strict' strips HTML from diagram labels and disables click-bound scripts — the control that
# holds the one untrusted innerHTML sink (the rendered SVG). 'loose' reopens CVE-2025-54881-class
# stored XSS from a collaborator's diagram. Pin it.
if grep -REn "securityLevel:[[:space:]]*['\"]loose['\"]" ui/src; then
    echo "  FAIL: Mermaid securityLevel 'loose' reopens diagram-label XSS; keep it 'strict'."
    fail=1
fi

echo "[check] a crate that vendors code under an undeclared licence must be in the notices..."
# The gap this defends, and why it is here rather than in deny.toml:
#
# `cargo deny` and `cargo tree` both read a crate's declared `license` field. A crate that
# vendors third-party source under a *different* licence declares only its own, and both
# believe it. `libgit2-sys` is exactly that: it declares `MIT OR Apache-2.0` — true of its
# Rust wrapper — while vendoring libgit2, which is GPL-2.0-only with a linking exception.
#
# cargo-deny cannot be taught this. `[[licenses.clarify]]` is silently inert when a crate has
# a valid `license` field (verified against 0.20.2), so `cargo deny` reports `licenses ok` and
# always will. See the long comment in `deny.toml`.
#
# So this is the gate. `ci/third-party.sh` generates the notices that legally must travel with
# a shipped binary; if such a crate enters the graph without its override there, the artifact
# ships a notice claiming it is MIT/Apache. That is the failure this catches.
#
# Keyed on Cargo.lock so it costs a grep, and so it fires on an *optional* dependency too —
# `fm-core/native-git` is off by default, but the day someone builds with it the notice has to
# already be right.
for crate in libgit2-sys openssl-src; do
    if grep -q "^name = \"$crate\"\$" Cargo.lock 2>/dev/null; then
        if ! grep -q "$crate" ci/third-party.sh; then
            echo "  FAIL: $crate is in Cargo.lock but ci/third-party.sh has no licence override"
            echo "        for it, so THIRD-PARTY.md would state its declared licence, which is"
            echo "        not the whole truth. Add the override before shipping a binary."
            fail=1
        fi
    fi
done

echo "[check] TLS uses the ring backend, never aws-lc-rs (licence + toolchain)..."
# `rustls` and `rcgen` both DEFAULT to `aws-lc-rs`, whose licence is
# `ISC AND (Apache-2.0 OR ISC) AND OpenSSL` — and `OpenSSL` is not in deny.toml's allow list, so
# `cargo deny` fails. It also wants cmake and a C toolchain pixi does not provide, so the failure
# on a fresh machine is a confusing build error rather than a licence message.
#
# Both are pinned to `ring` in fm-serve/Cargo.toml. This guards the pin, because the way it breaks
# is silent: any future dependency that enables the default features of either crate drags
# aws-lc-rs back into the graph without touching a line we would notice.
if grep -q '^name = "aws-lc-rs"$' Cargo.lock 2>/dev/null; then
    echo "  FAIL: aws-lc-rs is in Cargo.lock. Something enabled the default features of rustls"
    echo "        or rcgen. Pin them to \`default-features = false, features = [\"ring\", ...]\`."
    echo "        See crates/fm-serve/Cargo.toml."
    fail=1
fi

echo "[check] fm-serve still builds with no default features (std-only, no TLS, no agent)..."
# The crate's own doc promises "std-only networking", and `tls` is the one feature that carries a
# dependency. That promise was never actually gated — this is the compile-time proof, and the same
# guarantee the `agent` feature gives for subprocesses. Cheap: a `check`, not a build.
if command -v cargo >/dev/null 2>&1; then
    if ! cargo check -q -p fm-serve --no-default-features 2>/dev/null; then
        echo "  FAIL: \`cargo check -p fm-serve --no-default-features\` does not compile."
        echo "        Something outside the \`tls\`/\`agent\` features now depends on them."
        fail=1
    fi
else
    echo "  (skipped: cargo not on PATH)"
fi

echo "[check] the mobile study agent stays behind the 'agent' feature (notes-only pays nothing)..."
# The desktop core proves "rm -rf agents/ is byte-identical" with fm-serve's `agent` feature; the
# mobile shell must give the same guarantee, or a notes-only APK silently links the whole model
# runner. `pixi run ci` has no Android toolchain (ruling 3), so this is a structural grep rather
# than a --no-default-features build: the two agent crates must be `optional` (else they stay in the
# graph with the feature off), and lib.rs must gate `mod agent` behind the feature.
mobile_toml=mobile/src-tauri/Cargo.toml
mobile_lib=mobile/src-tauri/src/lib.rs
for dep in fm-agent fm-agent-run; do
    line=$(grep -E "^${dep}[[:space:]]*=" "$mobile_toml" 2>/dev/null || true)
    if [ -z "$line" ]; then
        echo "  FAIL: $mobile_toml has no '$dep = ...' line to check."
        fail=1
    elif ! printf '%s' "$line" | grep -q 'optional = true'; then
        echo "  FAIL: $dep must be 'optional = true' in $mobile_toml, or a --no-default-features"
        echo "        notes-only APK still links the agent. Put it behind the 'agent' feature."
        fail=1
    fi
done
if ! grep -Eq '^default[[:space:]]*=[[:space:]]*\[[^]]*"agent"' "$mobile_toml" 2>/dev/null; then
    echo "  FAIL: $mobile_toml must keep 'default = [\"agent\"]' so the shipped APK has the agent."
    fail=1
fi
# The gate itself: `mod agent;` must be preceded by the cfg, never bare.
if grep -Eq '^[[:space:]]*mod agent;' "$mobile_lib" 2>/dev/null \
    && ! grep -B1 -E '^[[:space:]]*mod agent;' "$mobile_lib" | grep -q 'cfg(feature = "agent")'; then
    echo "  FAIL: 'mod agent;' in $mobile_lib is not gated by #[cfg(feature = \"agent\")]."
    fail=1
fi

echo "[check] the Android setup hook: no panic path, and the store reachable before slow work..."
# **This is the only thing in `pixi run ci` that looks at the phone's startup at all**, so it is
# checked structurally rather than not at all: `pixi run ci` has no Android toolchain (ruling 3) and
# the mobile crate cannot even compile here (it is workspace-excluded and its desktop tauri backend
# wants webkit2gtk, which this env does not carry). Everything below is a real defect that shipped.
#
# The shape of the bug it pins (the owner's phone, 2026-07-31 — "gray screen on the first open, fine
# on the second"): Tauri builds the webview BEFORE this hook runs, so the page is already invoking
# while the hook works; the event loop that delivers a reply does not start until the hook returns;
# and a `?` inside it panics that thread, leaving a live webview with no backend behind it. That is
# not a crash anyone can report — it is a screen that never paints.
# 1) Neither the setup closure nor `boot` may fail upward or panic: the phone must end up with a
#    backend that can *say* what went wrong, which is what `BOOT` holds.
for region in 'setup' 'boot'; do
    case "$region" in
        setup) body=$(awk '/\.setup\(\|app\| \{/{on=1} on{print} on && /^        \}\)/{exit}' "$mobile_lib" 2>/dev/null || true)
               what='the Android setup hook' ;;
        boot)  body=$(awk '/^fn boot\(/{on=1} on{print} on && /^\}/{exit}' "$mobile_lib" 2>/dev/null || true)
               what='fn boot' ;;
    esac
    if [ -z "$body" ]; then
        echo "  FAIL: cannot find $what in $mobile_lib — this guard can no longer see the startup"
        echo "        path. Re-anchor it rather than deleting it."
        fail=1
        continue
    fi
    # `catch_unwind`'s own name is allowed to mention panics; a `?`/unwrap/expect is not.
    if printf '%s\n' "$body" | grep -vE 'catch_unwind|AssertUnwindSafe|^ *//' \
        | grep -nE '\?;|\.unwrap\(\)|\.expect\(|panic!\(|unreachable!' ; then
        echo "  FAIL: $what (lines above) must not use ?/unwrap/expect/panic — a failure there"
        echo "        panics the shell's thread and leaves a webview with no backend, which the user"
        echo "        sees as a blank screen and cannot report. Record it in BOOT and let every"
        echo "        command answer with the reason instead."
        fail=1
    fi
done
# 2) The store must be published before the optional work, or every millisecond of cert-store
#    reading and model launching is a millisecond the UI has no data and cannot say why.
manage_at=$(grep -n '\.manage(\|Manager::manage(' "$mobile_lib" | head -1 | cut -d: -f1)
if [ -z "$manage_at" ]; then
    echo "  FAIL: $mobile_lib never manages the vault store — no command can ever be answered."
    fail=1
else
    for slow in 'install_ca_bundle(&' 'agent::start(' 'ca_bundle::install'; do
        at=$(grep -nF "$slow" "$mobile_lib" | head -1 | cut -d: -f1 || true)
        if [ -n "$at" ] && [ "$at" -lt "$manage_at" ]; then
            echo "  FAIL: '$slow' runs before the store is managed in $mobile_lib. The webview is"
            echo "        already asking by then, so this is dead time the user spends looking at a"
            echo "        startup screen. Keep it after the manage(), off the critical path."
            fail=1
        fi
    done
fi
# 3) The reason has to be recorded somewhere the commands can read it, and a retry has to be able
#    to genuinely retry — a frozen reason is a "Try again" button that cannot help.
if ! grep -q 'static BOOT' "$mobile_lib"; then
    echo "  FAIL: $mobile_lib has no BOOT state — a failed startup then answers commands with"
    echo "        tauri's 'state not managed for field \`app\`' instead of what actually went wrong."
    fail=1
fi
if grep -q 'static BOOT:.*OnceLock' "$mobile_lib"; then
    echo "  FAIL: BOOT must not be a OnceLock — that freezes the first failure for the life of the"
    echo "        process, so the UI's retry re-asks and gets the same stale sentence forever."
    fail=1
fi

echo "[check] every Android IPC command is (async) — a blocking one freezes the screen..."
# **The single biggest cause of "the phone app is unusable".** Three facts compose:
#   1. Android never gets Tauri's async custom-protocol IPC (`canUseCustomProtocol = osName !==
#      'android'`), so it falls back to `window.ipc.postMessage`.
#   2. That is an `@JavascriptInterface` method and wry runs the handler INLINE — a JS->Java bridge
#      call is synchronous, so the page's JS thread is parked until Rust returns.
#   3. A plain `#[tauri::command]` is `ExecutionContext::Blocking` — the body runs before the call
#      returns.
# Net: the UI could not paint for the duration of *any* command, which is why every other cost on
# this platform presented as a freeze rather than as latency. `(async)` makes the macro spawn and
# return immediately.
#
# Structural grep for the same reason as the guards above: `pixi run ci` has no Android toolchain
# and the mobile crate is workspace-excluded, so nothing here compiles it. One forgotten `(async)`
# on a new command silently reintroduces the freeze for that command only — the hardest kind to
# attribute, which is exactly why it is worth a line in CI.
#
# Anchored to the start of the line so the attribute is matched and prose about it is not: this
# file's own doc comments discuss `#[tauri::command]` by name, and an unanchored grep failed on
# them — a guard that cannot be explained in a comment beside itself is a guard people delete.
#
# **Every `.rs` in the crate, and every spelling of the attribute.** The first version of this
# guard matched only the exact literal `#[tauri::command]` in `lib.rs`, which an adversarial review
# showed has two false negatives that both reintroduce the freeze with CI green: a command carrying
# any other argument (`#[tauri::command(rename_all = "snake_case")]`) is blocking and was not
# matched, and a command added to `agent.rs` — or any future module — was not looked at.
#
# So: scan the whole crate, match any `#[tauri::command` attribute, and require a bare `async`
# among its arguments.
for f in mobile/src-tauri/src/*.rs; do
    [ -f "$f" ] || continue
    bad_cmds=$(grep -nE '^[[:space:]]*#\[tauri::command(\]|\()' "$f" 2>/dev/null \
        | grep -vE '#\[tauri::command\(([^)]*,[[:space:]]*)?async[[:space:]]*[,)]' || true)
    if [ -n "$bad_cmds" ]; then
        echo "$bad_cmds" | sed "s|^|  $f:|"
        echo "  FAIL: the commands above are blocking (#[tauri::command] with no 'async'). On"
        echo "        Android that parks the WebView's JS thread for the whole call and the app"
        echo "        freezes. Use #[tauri::command(async)] — on the sync fn, not by making it"
        echo "        'async fn', so no MutexGuard can ever be held across an .await."
        fail=1
    fi
done
if ! grep -rqE '^[[:space:]]*#\[tauri::command\(([^)]*,[[:space:]]*)?async[[:space:]]*[,)]' \
    mobile/src-tauri/src 2>/dev/null; then
    echo "  FAIL: mobile/src-tauri/src exposes no async #[tauri::command] at all — this guard can"
    echo "        no longer see the IPC surface. Re-anchor it rather than deleting it."
    fail=1
fi

echo "[check] the boot gate has a screen for every state (no blank first paint)..."
# The other half of the same bug: `App.svelte`'s gate rendered NOTHING while `vaults === null`, so
# "the backend has not answered yet" and "the backend is dead" were one screen with no words on it.
# The behavioural tests live in `ui/src/App.boot.test.ts` (driven against a mock that can refuse,
# stall and hang); this grep is what stops the *screen* being deleted and the tests being deleted
# with it, since a gate with a missing branch is a one-line regression.
gate=ui/src/App.svelte
if ! grep -q 'Starting' "$gate"; then
    echo "  FAIL: $gate no longer renders <Starting>. Some gate state now paints an empty document,"
    echo "        which on a phone is an unreportable blank screen — there is no console, no stdout"
    echo "        and no logcat there (known-issues.md). Keep a screen for every state."
    fail=1
fi
if [ ! -f ui/src/App.boot.test.ts ]; then
    echo "  FAIL: ui/src/App.boot.test.ts is gone — the slow/refusing/silent-backend cases are the"
    echo "        only tests that can see the blank-screen class of bug. Do not delete them."
    fail=1
fi

echo "[check] docs/context always-read layer: every doc pointer resolves..."
# The always-loaded layer (overview.md + features.md) is a router: it names on-demand files to
# pre-read per hot area. That forcing function is only real if the targets exist — an on-demand doc
# no one is pointed to is invisible, and a pointer to a moved/renamed file is worse than none. So
# every *.md reference in the always-read layer must resolve (relative to docs/context), and every
# `decisions.md#<subject>` the router cites must be a real subject in the decisions index.
missing=""
for ref in $(grep -hoE '[A-Za-z0-9._/-]+\.md' docs/context/overview.md docs/context/features.md | sort -u); do
    case "$ref" in
        ../*|*/MASTERPLAN.md) continue ;;   # outside the context tree; checked elsewhere
    esac
    [ -f "docs/context/$ref" ] || missing="$missing $ref"
done
if [ -n "$missing" ]; then
    echo "  FAIL: the always-read layer points at file(s) that do not exist:$missing"
    echo "        fix the link, or the router sends the next agent to a dead end."
    fail=1
fi
for subj in $(grep -hoE 'decisions\.md#[a-z-]+' docs/context/overview.md docs/context/features.md | sed -E 's/.*#//' | sort -u); do
    if ! grep -q "#${subj}\`" docs/context/decisions.md; then
        echo "  FAIL: the router cites decisions.md#${subj}, but decisions.md has no #${subj} subject."
        fail=1
    fi
done

echo "[check] the always-read context layer stays under its size budget..."
# The always-read layer (overview.md + features.md) is loaded every session, and the whole reason
# this structure exists is that an ever-growing always-loaded file is what makes an agent skim and
# ignore it. Enforce the ceiling rather than hope for it — re-bloat cannot ship green. If this
# trips, the fix is to MOVE detail into an on-demand file and add a router row, never to raise the
# cap. (~400 lines is the researched budget; the seam mechanics stay in, everything else earns it.)
always_read_budget=420
always_read_lines=$(cat docs/context/overview.md docs/context/features.md | wc -l)
if [ "$always_read_lines" -gt "$always_read_budget" ]; then
    echo "  FAIL: overview.md + features.md = ${always_read_lines} lines, over the ${always_read_budget} budget."
    echo "        Move detail to an on-demand doc + add a router row; do not raise the cap."
    fail=1
fi

echo "[check] known-issues.md has no zombie 'FIXED' entries (delete when fixed)..."
# The file's own rule is "when you fix something, delete its entry." A struck-through '~~…~~ — FIXED'
# bullet is a fix that never got deleted — pure bloat, and the single biggest source of it here. The
# story of a fix lives in git + sessions/; a durable lesson belongs in known-issues' traps or in
# decisions.md, not in a strikethrough. Enforce the delete.
if grep -nE '~~.*~~[^*]*FIXED' docs/context/known-issues.md; then
    echo "  FAIL: delete the struck-through FIXED entr(y/ies) above — keep only the durable lesson,"
    echo "        in the traps section or decisions.md. The fix's story is in git + sessions/."
    fail=1
fi

echo "[check] the release sheet points at paths the release actually stages..."
# The archive's README is read by the one person who can check nothing: someone holding a .zip,
# offline, with no way to discover that a folder was renamed after the sheet was written. This is
# the docs/context router check one layer out, and the difference is who pays — a stale router
# costs a maintainer one grep, a stale release sheet costs a user the app.
#
# The chain has three links since 2026-08-28, because a tester opened `manual/`, met ~50 files and
# could not tell which to click:
#     README.txt  ->  Manual.html  ->  manual/index.html
# Each link is checked against the step that actually stages it. Break any one and the user lands
# on a path that is not there.
if ! grep -qF 'Manual.html' packaging/README-release.txt; then
    echo "  FAIL: packaging/README-release.txt no longer sends the reader to Manual.html —"
    echo "        which is the only unambiguous way into the manual from an unzipped folder."
    fail=1
fi
if ! grep -qF 'manual/index.html' packaging/launcher/Manual.html; then
    echo "  FAIL: packaging/launcher/Manual.html no longer points at manual/index.html, so the"
    echo "        one door into the manual opens onto nothing."
    fail=1
fi
for p in 'manual/index.html' 'manual/source' 'Manual.html' 'README.txt'; do
    if ! grep -qF "$p" .github/workflows/release.yml; then
        echo "  FAIL: .github/workflows/release.yml no longer stages $p, but the release still"
        echo "        promises it. Change both, or neither."
        fail=1
    fi
done

# The top level is the whole point of the 2026-08-28 repackaging: a first-time user must meet a
# door, not an inventory. `program/` is where the machinery went, and a binary copied back to the
# top level would quietly undo that — with nothing failing, because the launcher would still work
# on the developer's machine where they never looked at the folder.
if ! grep -qF 'stage/${DIR}/program/' .github/workflows/release.yml; then
    echo "  FAIL: release.yml no longer stages the binaries under program/. The archive's top"
    echo "        level is what a non-technical user sees; keep it a door, not an inventory."
    fail=1
fi

echo "[check] the libgit2 exception is still mobile-only, in fact and not just in prose..."
# `deny.toml` says it three times — "ONE named exception: libgit2, for mobile only" — and **nothing
# defends it**. `cargo deny` reports `licenses ok` for libgit2 and always will: `libgit2-sys`
# declares "MIT OR Apache-2.0", saying nothing about the ~230k lines of GPL C it vendors, and
# `[[licenses.clarify]]` was tried and removed because it never fires. So the whole gate on
# widening that exception is documentary.
#
# That is the failure this check exists for, and it is not hypothetical: turning on
# `fm-serve/native-git` is one line, changes no behaviour on a machine that has git, breaks no
# test — and silently makes deny.toml's central claim false. Scope drift with nothing red.
#
# Widening the exception may well be right (the owner's Track M ruling 1 names the desktop as the
# destination). This does not forbid it. It requires that the prose move at the same time, by
# failing until someone has been back to `deny.toml` and `decisions.md#git`.
if command -v cargo >/dev/null 2>&1; then
    for crate in fm-serve fm-cli; do
        if cargo tree -p "$crate" -e normal 2>/dev/null | grep -q 'git2 v'; then
            echo "  FAIL: $crate pulls git2 in a DEFAULT build, but deny.toml still says the"
            echo "        libgit2 exception is 'mobile only'. Widening it is allowed — but write"
            echo "        the dated decisions.md#git entry and fix deny.toml's wording first,"
            echo "        because no other gate in this repo will notice (cargo deny cannot see"
            echo "        libgit2's real licence, and never will)."
            fail=1
        fi
    done
    # The guard must fail loudly if it can no longer see what it measures — the same rule the
    # Android boot checks above are held to. If `native-git` stops pulling git2, this check has
    # been silently measuring nothing.
    # Anchored on fm-core, whose `native-git` feature is the long-standing one the mobile exception
    # is built on — not on whichever shipped crate happens to expose a passthrough this week.
    if ! cargo tree -p fm-core --features native-git -e normal 2>/dev/null | grep -q 'git2 v'; then
        echo "  FAIL: this check can no longer see git2 even with native-git enabled, so it is"
        echo "        measuring nothing. Re-anchor it rather than deleting it."
        fail=1
    fi
fi

if [ "$fail" -eq 0 ]; then
    echo "all architectural checks passed."
fi
exit "$fail"
