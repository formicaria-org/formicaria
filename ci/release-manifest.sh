#!/bin/sh
# Write a release's manifest: which files it holds, which build target each one is for, what each
# hashes to, and how large it is. `crates/fm-update` reads exactly this shape.
#
# **What gets signed is this file, not a bare hash.** A signature over a loose sha256 is replayable:
# whoever chooses which signed bytes a client sees can serve last year's genuine hash and install a
# version with a known hole. Binding `version` and `target` into the signed bytes is what makes "this
# is the artifact I asked for" checkable. The signature itself is made by `ci/release-sign.sh`, in a
# separate job that never sees the artifacts — see `decisions.md#toolchain` (2026-09-11).
#
#   sh ci/release-manifest.sh <dist-dir> <tag>     →  <dist-dir>/formicaria-<tag>.manifest.json
set -eu

dist=${1:?usage: release-manifest.sh <dist-dir> <tag>}
tag=${2:?usage: release-manifest.sh <dist-dir> <tag>}

# **Only a version the updater will compare.** `Version::parse` accepts `vMAJOR.MINOR.PATCH` and
# nothing else — no `-rc1`, no `dev-<sha>` — so a manifest for anything else is one no installed copy
# would ever act on. Refusing here makes that a failed job rather than a silently useless file.
if ! printf '%s\n' "$tag" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+$'; then
    echo "release-manifest: '$tag' is not a release version (vMAJOR.MINOR.PATCH)." >&2
    exit 1
fi

sha256() {
    if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -d' ' -f1
    else shasum -a 256 "$1" | cut -d' ' -f1
    fi
}

out="$dist/formicaria-$tag.manifest.json"
tmp=$(mktemp)
found=0
printf '{\n  "version": "%s",\n  "artifacts": [' "$tag" > "$tmp"
# The release naming is the contract: `formicaria-<tag>-<target>.<ext>`. Selecting by that prefix is
# also what keeps `unsigned-*` intermediates out without a second rule to forget.
for f in "$dist"/formicaria-"$tag"-*; do
    [ -f "$f" ] || continue
    base=$(basename "$f")
    case "$base" in
        *.tar.gz) stem=${base%.tar.gz} ;;
        *.zip)    stem=${base%.zip} ;;
        *.apk)    stem=${base%.apk} ;;
        *.ipa)    stem=${base%.ipa} ;;
        *)        continue ;;
    esac
    target=${stem#formicaria-"$tag"-}
    # Hand-written JSON is only safe over names we control, so check that they are.
    case "$base$target" in
        *[!A-Za-z0-9._-]*)
            echo "release-manifest: refusing '$base' — its name is not one this script can write safely." >&2
            rm -f "$tmp"; exit 1 ;;
    esac
    [ "$found" -eq 0 ] || printf ',' >> "$tmp"
    found=$((found + 1))
    printf '\n    {"target": "%s", "file": "%s", "sha256": "%s", "size": %s}' \
        "$target" "$base" "$(sha256 "$f")" "$(wc -c < "$f" | tr -d ' ')" >> "$tmp"
done
printf '\n  ]\n}\n' >> "$tmp"

if [ "$found" -eq 0 ]; then
    echo "release-manifest: no formicaria-$tag-* artifacts in $dist" >&2
    rm -f "$tmp"; exit 1
fi
mv "$tmp" "$out"
echo "release-manifest: $found artifacts -> $out"
