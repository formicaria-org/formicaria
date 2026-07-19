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
if grep -REn 'std::(fs|path)|std::io::[A-Za-z]*File' crates/fm-query/src | grep -v '//'; then
    echo "  FAIL: fm-query references a filesystem API — the seam is broken."
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
for crate in libgit2-sys; do
    if grep -q "^name = \"$crate\"\$" Cargo.lock 2>/dev/null; then
        if ! grep -q "$crate" ci/third-party.sh; then
            echo "  FAIL: $crate is in Cargo.lock but ci/third-party.sh has no licence override"
            echo "        for it, so THIRD-PARTY.md would state its declared licence, which is"
            echo "        not the whole truth. Add the override before shipping a binary."
            fail=1
        fi
    fi
done

if [ "$fail" -eq 0 ]; then
    echo "all architectural checks passed."
fi
exit "$fail"
