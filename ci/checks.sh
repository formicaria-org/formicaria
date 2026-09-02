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

echo "[check] every hand-written vcs arm reaches both backends..."
# The guard above walks `route!(…)` entries only — so a **hand-written** arm is invisible to it,
# and two of them exist because the macro's by-value arm cannot express a `&[PathBuf]`. That is
# exactly how `commit_all_as` shipped calling the subprocess unconditionally: routed nowhere,
# grepped by nothing, and reached on the one device with no `git` binary. It failed silently
# (`dispatch.rs` discards the result) and the agent's commits landed under the vault's default
# identity instead of the model's, for six weeks.
#
# Note the earlier `fm_core::git::` guard cannot see this either: it excludes `crates/fm-core/src/`,
# because that is where both backends legitimately live. This check is that hole.
#
# The exception is genuine: `available()` asks "is there a git BINARY", which is the one question
# libgit2 has no answer to — it is how `native()` decides in the first place.
unrouted=""
for fn in $(grep -oE '^[^/]*crate::git::[a-z_]+' crates/fm-core/src/vcs.rs \
    | grep -oE 'crate::git::[a-z_]+' | sed 's/^crate::git:://' | sort -u); do
    [ "$fn" = "available" ] && continue
    grep -q "crate::git_native::${fn}\b" crates/fm-core/src/vcs.rs || unrouted="$unrouted $fn"
done
if [ -n "$unrouted" ]; then
    echo "  FAIL: vcs.rs calls these on the subprocess backend with no libgit2 arm:$unrouted"
    echo "        Every operation vcs.rs exposes must reach BOTH backends, or it is an operation"
    echo "        the phone cannot perform — and the phone is the device nobody can debug."
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

echo "[check] the app asks its questions without git vocabulary..."
# The owner's standing constraint on the review surface: "this has to work without the user knowing
# what a commit is." The supervision record is only as good as the reason a person types into it, and
# a field labelled in plumbing vocabulary is one nobody answers.
#
# Scoped to **labels and placeholders** — the microcopy that asks a person for something — because
# that is where the guard can be true. It deliberately does NOT cover button text or status messages:
# `ProposalReview.svelte` already says "Accept & merge" and "Merged into main", those pre-date this
# rule, and cleaning them up is a separate decision. The point is to stop a *fifth* one appearing in
# the place a user is being asked to write.
bad_copy=$(
    { grep -rhoE 'placeholder="[^"]*"' ui/src --include=*.svelte
      grep -rhoE '<label[^>]*>[^<]*' ui/src --include=*.svelte | sed 's/<label[^>]*>//'; } \
    | grep -inE 'commit|branch|merge|trailer|\bHEAD\b|repository' || true
)
if [ -n "$bad_copy" ]; then
    echo "  FAIL: user-facing microcopy uses git vocabulary:"
    printf '        %s\n' "$bad_copy"
    echo "        Ask the question in the user's words; the plumbing stays underneath."
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
# The update script belongs in the same chain, and the stake is higher than a broken link: the
# sheet tells someone whose notes are in another folder to double-click it. Promised and not
# staged, they are left copying folders by hand at exactly the moment they believe their work is
# gone. The base name only — the extension differs per platform.
for p in 'manual/index.html' 'manual/source' 'Manual.html' 'README.txt' 'Update from an older folder'; do
    if ! grep -qF "$p" .github/workflows/release.yml; then
        echo "  FAIL: .github/workflows/release.yml no longer stages $p, but the release still"
        echo "        promises it. Change both, or neither."
        fail=1
    fi
done

# **A media query adds no specificity — so a base rule can silently beat one.**
#
# The view rail shipped invisible at every width and nobody noticed for two commits. It had
# `display: flex` inside `@media (min-width: 60rem)` and `display: none` in a base rule *later* in
# the file. Equal specificity, so the later one won, everywhere. jsdom applies no CSS, so the
# component test that finds those buttons passed the whole time; grepping the bundle for the class
# name only proved the markup shipped.
#
# The rule this encodes: the rail is visible by default and hidden in exactly one place — the
# narrow media query. A bare `display: none` on it is the bug coming back.
if [ -f ui/src/App.svelte ]; then
    if awk '/^  \.panel-views \{/,/^  \}/' ui/src/App.svelte | grep -q 'display: none'; then
        echo "  FAIL: .panel-views has a bare 'display: none' outside a media query. That ties with"
        echo "        the media rule and wins on source order — which is how the rail shipped"
        echo "        invisible. Hide it in the narrow @media block instead."
        fail=1
    fi
fi

# **The chrome's placement is a fact about the window, not about the arrangement.**
#
# The sibling of the check above, and the same lesson from the other side. `[data-layout='single']
# .topbar .icon-btn { display: none }` outranked the rule that shows Help, Settings and the panel
# toggle in the wide panel — so choosing one-view-at-a-time on a desktop silently deleted three
# controls from the rail. Nobody hit it while `auto` was the default and almost nobody picked
# `single`; making `single` the default would have shipped it to everyone on first launch. jsdom
# applies no CSS, so no component test can see this.
#
# The rule: which panes are *visible* is the arrangement's business (`--cell`, `--viewbar`, set in
# one block). Where the chrome *sits*, and what it can afford to show, is the window's — say it in
# the width media query, where it applies whatever the user chose.
echo "[check] the chrome is placed by window width, never by the chosen arrangement..."
if [ -f ui/src/App.svelte ]; then
    if grep -nE "\[data-layout='(single|tiled)'\] \.(topbar|icon-btn|save-label|panel-views)" ui/src/App.svelte; then
        echo "  FAIL: a chrome rule above is keyed on the arrangement instead of the window width."
        echo "        That is how the desktop rail lost Help, Settings and its collapse toggle."
        echo "        Put it in the @media (max-width: 59.999rem) block instead."
        fail=1
    fi
fi

# **A later same-specificity rule must not reset the safe area with a shorthand.**
#
# The third arrival of one lesson. Twice it came through `display` — the view rail that shipped
# invisible, and the chrome rule that would have deleted Help and Settings from the desktop — and
# both have guards above. This time it came through `padding`: `.topbar` had
# `padding: … max(var(--safe-bottom), …) …` in the 59.999rem block and a bare
# `padding: 0.4rem 0.5rem` in the 40rem block. A phone matches both, media queries add no
# specificity, so the shorthand won and reset all four sides — leaving 6.4px of clearance under a
# 47px navigation bar. Back up, "get their changes" and Settings could not be tapped at all, and
# the rule that broke it had been correct for six weeks before the bar moved to the bottom.
#
# The rule: inside a narrower `.topbar` rule, set padding with longhands. The bottom belongs to the
# one rule that knows about the inset.
echo "[check] a narrow .topbar rule must not reset the safe area with a padding shorthand..."
if [ -f ui/src/App.svelte ]; then
    if awk '/@media \(max-width: 40rem\)/,/^  \}$/' ui/src/App.svelte \
        | awk '/^    \.topbar \{/,/^    \}/' \
        | grep -qE '^\s*padding:'; then
        echo "  FAIL: a .topbar rule in the 40rem block uses the 'padding' shorthand. It ties with"
        echo "        the safe-area rule above and wins on source order, resetting the bottom inset"
        echo "        — which is how the bottom bar ended up under the navigation buttons."
        echo "        Use padding-top / padding-inline and leave the bottom to the inset rule."
        fail=1
    fi
fi

# **What `fm-serve` looks for beside itself, the release must stage.**
#
# The assistant resolves `agent-serve` and `models.toml` from `current_exe().parent()` — never from
# `PATH`, never from the cwd. So the archive is the *only* place they can come from, and a release
# that stops staging either one produces an app that reports "this copy did not come with the
# assistant" with nothing wrong in any test. The same shape as the release-sheet check above, and
# the same stake: promised in the code, absent from the archive.
#
# **Comments are stripped before the workflow is searched**, and that is not fussiness: the first
# version of this check passed while the staging was deleted, because the *explanation* above the
# staging line still said "agent-serve". A guard satisfied by prose about the thing is a guard that
# reports on its own documentation.
echo "[check] the release stages what the assistant looks for beside the binary..."
release_code=$(grep -av '^[[:space:]]*#' .github/workflows/release.yml)
for p in agent-serve models.toml; do
    if grep -aq "$p" crates/fm-serve/src/agent.rs && ! printf '%s' "$release_code" | grep -aq "$p"; then
        echo "  FAIL: crates/fm-serve/src/agent.rs looks for '$p' beside the running binary, but"
        echo "        .github/workflows/release.yml never stages it. A downloaded copy would then"
        echo "        have no assistant at all, and nothing here would fail. Stage it, or stop"
        echo "        looking for it."
        fail=1
    fi
done

# **The phone answers the same status shape as the desktop.**
#
# `SettingsPanel.svelte` is rendered by BOTH shells — the mobile app points its webview at the same
# `ui/dist` — and it reads `st.installed` to decide between a switch and a reason. The phone once
# answered `{enabled, transcribe}` only, so `installed` was `undefined`, `undefined` is falsy, and
# the assistant row printed "not available" with an empty reason **on the one platform where the
# whole stack ships inside the APK**. One component reading two divergent shapes is a defect that
# compiles, ships and shows nothing in any test: jsdom sees neither shell.
echo "[check] the phone's agent_status answers every key the desktop's does..."
if [ -f mobile/src-tauri/src/lib.rs ] && [ -f crates/fm-serve/src/agent.rs ]; then
    for k in installed why transcribe_available enabled transcribe; do
        if ! grep -aq "\"$k\"" mobile/src-tauri/src/lib.rs; then
            echo "  FAIL: mobile/src-tauri/src/lib.rs never names \"$k\", which fm-serve's"
            echo "        /api/agent_status answers and SettingsPanel.svelte reads. The phone and"
            echo "        the desktop render the same component, so a key missing on one side is a"
            echo "        row that renders wrong on that platform only. Add it to agent_status."
            fail=1
        fi
    done
fi

# **The attachment ceiling is one number, written twice.**
#
# `GIT_ASSETS_CEILING` is the largest attachment this app will put into git. Rust is the
# authority — it refuses the setting and clamps the staging walk — and `ui/src/lib/size.ts` holds
# a copy purely so the form can warn before the backend refuses. If the two drift, the UI either
# warns about a limit that is fine or accepts one the backend will reject, and the user meets the
# disagreement as an error they cannot act on. There is no git-lfs here, so this number is what
# stands between an attachment and a push that fails after the commit is already made.
echo "[check] the attachment ceiling agrees between Rust and the UI..."
rs_ceiling=$(grep -aoE 'pub const GIT_ASSETS_CEILING: u64 = [0-9_]+' crates/fm-core/src/descriptor.rs \
    | grep -aoE '[0-9_]+$' | tr -d _)
ts_ceiling=$(grep -aoE 'export const GIT_ASSETS_CEILING = [0-9_]+' ui/src/lib/size.ts \
    | grep -aoE '[0-9_]+$' | tr -d _)
if [ -z "$rs_ceiling" ] || [ -z "$ts_ceiling" ]; then
    echo "  FAIL: could not read GIT_ASSETS_CEILING from both sides."
    echo "        Rust: '${rs_ceiling:-<not found>}' (crates/fm-core/src/descriptor.rs)"
    echo "        UI:   '${ts_ceiling:-<not found>}' (ui/src/lib/size.ts)"
    echo "        If either constant was renamed or reshaped, update this check with it."
    fail=1
elif [ "$rs_ceiling" != "$ts_ceiling" ]; then
    echo "  FAIL: the attachment ceiling disagrees — Rust says $rs_ceiling, the UI says $ts_ceiling."
    echo "        Rust is the authority (it refuses the setting and clamps the staging walk); the"
    echo "        UI copy exists only to warn first. Make ui/src/lib/size.ts match descriptor.rs."
    fail=1
fi

# **An overlay panel is bounded by the visible viewport, and it scrolls.**
#
# Every dialog in this app is `position: fixed` inside `.app`, which is `height: 100dvh;
# overflow: hidden` — the document itself never scrolls. So an overlay that outgrows the screen
# has no fallback whatsoever: its bottom rows are unreachable, on a phone and on a short desktop
# window alike. `BackupPanel` shipped with neither a `max-height` nor an `overflow` and the thing
# you could not reach was its primary button. `SettingsPanel` and the shared `.sheet` had the cap
# but wrote it in `vh`, which is the *tallest* the viewport ever gets — so with the URL bar out or
# the keyboard up they were still taller than the screen.
#
# Two rules, one grep each. Both are invisible to the test suite: jsdom computes no layout, so
# nothing in `ui/src` can assert a height or an overflow.
#
# Scope, stated rather than implied. The first sweeps `*.svelte` only — `app.css`'s
# `.read .asset-pdf { height: 70vh }` is a deliberate exception: it sizes an iframe *inside* an
# already-scrolling pane, not a surface bounded by the viewport. The second is a file-level
# grep, so it proves a panel component declares a scrollport *somewhere*, not that the right
# element carries it; it catches the defect that shipped (none at all), not a misplaced one.
# `-a` on every sweep below, and it is not decoration: `SkippedPanel.svelte` carried a literal
# NUL byte in a template literal, which made it `data` to `file(1)` — and **grep skips a binary
# file in silence**. This guard passed over it clean while it held a `76vh`. Any grep over
# `ui/src` that omits `-a` is making a claim it has not checked.
echo "[check] an overlay panel caps its height in dvh, not vh..."
if grep -ranE '^\s*(max-)?height:[^;]*[0-9]vh' ui/src --include='*.svelte' >/dev/null; then
    echo "  FAIL: an overlay caps its height in 'vh'. '100vh' is the tallest the viewport ever"
    echo "        gets, so with a retracting URL bar or an on-screen keyboard the panel is taller"
    echo "        than what you can see and its last rows are unreachable. Use the shared"
    echo "        overlay pattern in app.css ('max-height: 100%' inside a 100dvh layer), or 'dvh'."
    grep -ranE '^\s*(max-)?height:[^;]*[0-9]vh' ui/src --include='*.svelte'
    fail=1
fi

echo "[check] every overlay panel has somewhere to scroll..."
for f in ui/src/lib/BackupPanel.svelte ui/src/lib/SettingsPanel.svelte \
         ui/src/lib/HelpPanel.svelte ui/src/lib/SkippedPanel.svelte; do
    [ -f "$f" ] || continue
    if ! grep -qaE '^\s*overflow(-y)?:\s*(auto|scroll)' "$f"; then
        echo "  FAIL: $f is a fixed overlay with no 'overflow: auto' anywhere. Inside"
        echo "        '.app' (overflow: hidden) that means its content past the fold cannot be"
        echo "        reached by any gesture. Give the panel 'max-height: 100%' inside the"
        echo "        overlay's 100dvh box, and 'overflow-y: auto' — one scroll surface, on the panel."
        fail=1
    fi
done

# **The Appearance form is a list of tokens, never a language.**
#
# It re-values design tokens; it must never grow the ability to invent one, or write a selector or
# a media query — that is the query-builder this project has refused twice, arriving through the
# other door. The mechanical form of that rule: the token names the form can write live in
# `appearance.ts`'s SUPPORTED, and the component never names one in its own logic. (Its `<style>`
# block consumes tokens like any component, and its help text prints a few as examples; only the
# script is checked, because that is where a hardcoded list would actually take effect.)
if [ -f ui/src/lib/Appearance.svelte ]; then
    if awk '/<script/,/<\/script>/' ui/src/lib/Appearance.svelte | grep -qE -- '--[a-z]'; then
        echo "  FAIL: Appearance.svelte names a design token in its script. The set the form can"
        echo "        write belongs in appearance.ts (SUPPORTED), so the closed list has one home."
        fail=1
    fi
    if ! grep -q "SUPPORTED" ui/src/lib/appearance.ts; then
        echo "  FAIL: appearance.ts no longer declares SUPPORTED — the form's closed list is gone."
        fail=1
    fi

    # The manual promises these names will not move under a theme author. A promise the code has
    # quietly stopped keeping is worse than no promise, so the two lists are compared rather than
    # trusted: every token in SUPPORTED must appear in the page, and vice versa.
    missing=$(
        sed -n "/export const SUPPORTED/,/^];/p" ui/src/lib/appearance.ts |
            grep -oE "'[a-z0-9-]+'" | tr -d "'" |
            while read -r t; do
                grep -qF -- "\`--$t\`" docs/src/user/appearance.md || echo "$t"
            done
    )
    if [ -n "$missing" ]; then
        echo "  FAIL: the manual's token list is missing: $(echo "$missing" | tr '\n' ' ')"
        echo "        docs/src/user/appearance.md must list every name in appearance.ts SUPPORTED."
        fail=1
    fi
    extra=$(
        grep -oE '`--[a-z0-9-]+`' docs/src/user/appearance.md | tr -d '`' | sed 's/^--//' | sort -u |
            while read -r t; do
                sed -n "/export const SUPPORTED/,/^];/p" ui/src/lib/appearance.ts |
                    grep -qE "'$t'" || echo "$t"
            done
    )
    if [ -n "$extra" ]; then
        echo "  FAIL: the manual promises names the app does not support: $(echo "$extra" | tr '\n' ' ')"
        fail=1
    fi
fi

# The first note a new user reads has to actually be in the archive. It is also the one file whose
# failure is guaranteed to be invisible: the note loader is tolerant by design, so a note that does
# not parse does not error — it disappears, and the newcomer opens an empty notebook. `welcome_note.rs`
# proves the file parses; this proves the release still puts it where the app will look.
if ! grep -qF 'packaging/welcome/notes/.' .github/workflows/release.yml; then
    echo "  FAIL: release.yml no longer stages the welcome note into the vault. A first-time user"
    echo "        would open an empty notebook with nothing telling them what to do."
    fail=1
fi
if ! ls packaging/welcome/notes/*.md >/dev/null 2>&1; then
    echo "  FAIL: packaging/welcome/notes holds no note, but release.yml stages it."
    fail=1
fi

# The top level is the whole point of the 2026-08-28 repackaging: a first-time user must meet a
# door, not an inventory. `program/` is where the machinery went, and a binary copied back to the
# top level would quietly undo that — with nothing failing, because the launcher would still work
# on the developer's machine where they never looked at the folder.
if ! grep -qF 'stage/${DIR}/program/' .github/workflows/release.yml; then
    echo "  FAIL: release.yml no longer stages the binaries under program/. The archive's top"
    echo "        level is what a non-technical user sees; keep it a door, not an inventory."
    fail=1
fi

echo "[check] the libgit2 exception stays scoped to devices with no git binary..."
# `deny.toml` names one licence exception, and **nothing else defends its scope**. `cargo deny`
# reports `licenses ok` for libgit2 and always will: `libgit2-sys` declares "MIT OR Apache-2.0",
# saying nothing about the ~230k lines of GPL C it vendors, and `[[licenses.clarify]]` was tried
# and removed because it never fires. So the whole gate on widening it is documentary — which is
# exactly how scope drifts with nothing red.
#
# The boundary, as of 2026-08-28 (`decisions.md#git`): libgit2 ships where there is **no git
# binary** — Android, and Windows, which ships none. Linux and macOS desktops overwhelmingly have
# git, keep the subprocess backend, and must not pay for a vendored C library they never call.
#
# Checked against those two targets **explicitly, not against the host**: `pixi run ci` also runs
# on the Windows runner via `cross.yml`, where a host-resolved tree legitimately contains git2.
if command -v cargo >/dev/null 2>&1; then
    for target in x86_64-unknown-linux-gnu aarch64-apple-darwin; do
        for crate in fm-serve fm-cli; do
            if cargo tree -p "$crate" -e normal --target "$target" 2>/dev/null | grep -q 'git2 v'; then
                echo "  FAIL: $crate pulls git2 for $target, where a git binary is expected to exist."
                echo "        Widening the exception again is allowed — but move deny.toml's wording"
                echo "        and write the dated decisions.md#git entry first, because no other gate"
                echo "        here will notice (cargo deny cannot see libgit2's licence, and never"
                echo "        will)."
                fail=1
            fi
        done
    done
    # Windows is supposed to be INSIDE the exception now. If it stops pulling git2, the Windows
    # backup path has silently reverted to "git is not installed" — the bug this fixed.
    if ! cargo tree -p fm-serve -e normal --target x86_64-pc-windows-msvc 2>/dev/null | grep -q 'git2 v'; then
        echo "  FAIL: fm-serve does NOT pull git2 for Windows. Windows ships no git binary, so this"
        echo "        is a build with no history, no backup and no collaboration — see"
        echo "        decisions.md#git. (Or this check can no longer see what it measures, in which"
        echo "        case re-anchor it rather than deleting it.)"
        fail=1
    fi
fi

# ---------------------------------------------------------------------------------------------
# The assistant must not be able to keep formicaria alive.
#
# `fm-serve` exits after 90 s with nobody in touch, so closing the tab closes the app — and it
# learned "somebody is here" from any authenticated request. The study agent polls the vault every
# 1–5 s for as long as it runs, so while the assistant was on that window could never close: the
# app held itself open by asking whether it was open, and a multi-GB `llama-server` stayed resident
# with it until the machine was rebooted.
#
# The fix is one header the runner sends and the server checks. It is a **cross-crate agreement
# with no type behind it**: nothing depends on `fm-agent-run` (that is how "the core app never
# learns the agent exists" stays true), so the constant cannot be shared and a rename on one side
# would be silent — the runner would go back to holding the app open, and every test would pass.
# This is that missing compiler.
agent_header='X-Formicaria-Agent'
for f in crates/fm-agent-run/src/fmserve.rs crates/fm-serve/src/main.rs; do
    if ! grep -q "$agent_header" "$f"; then
        echo "  FAIL: $f no longer spells '$agent_header'."
        echo "        The runner sends it and fm-serve checks it; they are the same string in two"
        echo "        crates that cannot import from each other. If it moved, move it in both —"
        echo "        otherwise the assistant silently defeats auto-shutdown again."
        fail=1
    fi
done

# ---------------------------------------------------------------------------------------------
# "Audio transcription" must mean audio gets transcribed.
#
# `agent-serve.sh` starts whisper only when **both** its runtime binary and its model are staged.
# `fm-serve` predicts that in Rust so the settings row can refuse instead of storing a preference
# and answering `{"ok":true}` — which is what it did, and why a user could tick the box, restart,
# and find no transcription and no explanation anywhere.
#
# A shell script cannot export a predicate, so the condition is written twice. If the script starts
# looking for a different file, the Rust check goes on saying yes about a machine that says no.
#
# **Still two deciders after the app stopped using the shell (2026-09-02).** `fm-serve` now spawns
# `agent-serve` directly and decides whisper itself, but `agents/agent-serve.sh` is still what
# `pixi run agent-serve` runs, so the dev loop has its own copy of the same condition. The guard
# narrowed; it did not go away.
for name in whisper-server ggml-base.en.bin; do
    if ! grep -q "$name" agents/agent-serve.sh || ! grep -q "$name" crates/fm-serve/src/agent.rs; then
        echo "  FAIL: '$name' is named in only one of agents/agent-serve.sh and"
        echo "        crates/fm-serve/src/agent.rs. They are the same condition in two languages:"
        echo "        the script decides whether whisper starts, the Rust decides whether the"
        echo "        settings row offers the switch. Out of step, the row promises what the script"
        echo "        will not do."
        fail=1
    fi
done

# ---------------------------------------------------------------------------------------------
# The front page has to point at the app, and at the right copy of it.
#
# `README.md` is the first thing anyone reads and the only install sheet a *browser* can see — the
# archive's own README is unreachable until after the download. It had drifted three ways at once:
# a releases link to an org that no longer owns the repo (so the download button was broken), a
# `./fm-serve` that moved into `program/` in August, and an `xattr` instruction that exists nowhere
# else in the project. The guard above covers `README-release.txt` and stopped ten lines short of
# the file more people read.
echo "[check] README.md points at the app that actually ships..."
if grep -q 'singhbal-baljinder/formicaria' README.md; then
    echo "  FAIL: README.md links to github.com/singhbal-baljinder/formicaria."
    echo "        The repository moved to formicaria-org; that link is a 404, and it is the"
    echo "        download button on the front page."
    fail=1
fi
if grep -qE '^\./fm-serve|`\./fm-serve`|\$ \./fm-serve' README.md; then
    echo "  FAIL: README.md tells the reader to run ./fm-serve. Since 2026-08-28 the binaries are"
    echo "        staged under program/ (see release.yml), so that path does not exist in an"
    echo "        unpacked archive."
    fail=1
fi

# ---------------------------------------------------------------------------------------------
# No document may still teach the macOS bypass Apple removed.
#
# Until Sequoia, right-click -> Open let a user past Gatekeeper, and every page we ship said so.
# Apple removed it. The dialog a blocked app now produces offers **Done** and **Move to Trash** —
# so a reader following our instruction, hunting for the "Open" we promised, is looking at a
# destructive button and has been told to expect a dialog and proceed. Documentation that steers a
# first-time user into deleting the product is worse than none, and it is invisible from here
# because nobody in this project runs macOS.
if grep -rniE 'right.?click.{0,40}(and choose|then|->|→).{0,10}\*{0,2}open'         docs/src/user/ packaging/README-release.txt README.md 2>/dev/null         | grep -vi 'removed it\|worked on macOS versions before\|Older instructions' >/dev/null; then
    echo "  FAIL: a shipped document still tells macOS users to right-click and choose Open."
    echo "        Apple removed that bypass in Sequoia; the dialog now offers only Done and"
    echo "        Move to Trash. Point at System Settings -> Privacy & Security -> Open Anyway,"
    echo "        and say plainly not to click Move to Trash."
    fail=1
fi

if [ "$fail" -eq 0 ]; then
    echo "all architectural checks passed."
fi
exit "$fail"
