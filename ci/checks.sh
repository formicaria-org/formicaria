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

if [ "$fail" -eq 0 ]; then
    echo "all architectural checks passed."
fi
exit "$fail"
