#!/bin/sh
# Move this app to the newest Myrme release it can link — `pixi run myrme-bump`.
#
# Myrme, the assistant, is linked from https://github.com/formicaria-org/myrme by a pinned tag
# (`decisions.md#agent`, 2026-10-18). Its releases carry a contract version; this app states the
# one it is written for (`MYRME_CONTRACT` in crates/fm-serve/src/agent.rs and the phone's agent.rs)
# and checks it at compile time. So "the newest release that builds" is "the newest compatible
# release": the script tries the release tags from the newest down to the current pin, keeps the
# first that builds, and moves all three pins (fm-serve, agent-serve, the phone shell) and both
# lockfiles to it. Releases only, never a release candidate or a branch: **a release is a tag with a
# published GitHub Release page**, which Myrme's CI publishes only once its gate and integration
# test pass on the tag (Myrme `decisions.md`, 2026-10-19). A bare tag is skipped.
#
# It does not run the gate: run `pixi run ci` next, and read Myrme's docs/context/releases.md for
# what changed. Nothing is committed.
set -eu
REPO=https://github.com/formicaria-org/myrme.git
MANIFESTS="crates/fm-serve/Cargo.toml crates/agent-serve/Cargo.toml mobile/src-tauri/Cargo.toml"
current=$(sed -n 's/.*myrme\.git", tag = "\(v[0-9.]*\)".*/\1/p' crates/fm-serve/Cargo.toml | head -1)
[ -n "$current" ] || { echo "myrme-bump: no Myrme pin found in crates/fm-serve/Cargo.toml" >&2; exit 1; }
for f in $MANIFESTS; do
    grep -q "myrme.git\", tag = \"$current\"" "$f" || { echo "myrme-bump: $f does not pin $current" >&2; exit 1; }
done

tags=$(git ls-remote --tags --refs "$REPO" 'v*' | sed 's#.*refs/tags/##' \
    | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | sort -V)
echo "myrme-bump: pinned $current; releases: $(echo $tags)"

# Published and not a prerelease: GitHub's API answers 404 for a tag without a page (or a draft).
released() {
    curl -sf "https://api.github.com/repos/formicaria-org/myrme/releases/tags/$1" \
        | grep -q '"prerelease": *false'
}

pin() {
    for f in $MANIFESTS; do sed -i.bak "s/myrme\.git\", tag = \"v[0-9.]*\"/myrme.git\", tag = \"$1\"/" "$f" && rm -f "$f.bak"; done
    # A changed tag is re-resolved by any resolve; naming the packages (`cargo update -p`) fails when
    # the lockfile holds them from somewhere else, such as a developer's local `[patch]`.
    cargo metadata -q --format-version 1 >/dev/null
    (cd mobile/src-tauri && cargo metadata -q --format-version 1 >/dev/null)
}

for tag in $(echo "$tags" | sort -rV); do
    [ "$tag" = "$current" ] && break
    if ! released "$tag"; then
        echo "myrme-bump: $tag has no published release page (yet, or its CI failed) — skipped."
        continue
    fi
    echo "myrme-bump: trying $tag..."
    pin "$tag"
    if cargo check -q -p fm-serve -p fm-agent-serve 2>/tmp/myrme-bump.$$; then
        rm -f /tmp/myrme-bump.$$
        echo "myrme-bump: pinned $tag (was $current). Next: pixi run ci, then commit."
        exit 0
    fi
    # Another contract major says so in words; a release from before contracts were numbered
    # (0.1.x) has no contract constants at all. Either way it is not for this app: skip it.
    if grep -q "speaks another contract" /tmp/myrme-bump.$$; then
        echo "myrme-bump: $tag speaks another contract — skipped."
    elif grep -q "CONTRACT_MAJOR\|CONTRACT_MINOR" /tmp/myrme-bump.$$; then
        echo "myrme-bump: $tag is from before contracts were numbered — skipped."
    else
        echo "myrme-bump: $tag does not build here — skipped; its first error:"
        grep -m1 "^error" /tmp/myrme-bump.$$ | sed 's/^/    /'
    fi
    rm -f /tmp/myrme-bump.$$
done
pin "$current"
echo "myrme-bump: $current is already the newest release this app can link."
