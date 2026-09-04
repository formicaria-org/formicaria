#!/bin/sh
# Assert the committed THIRD-PARTY.md still describes the tree it claims to describe.
#
# ## Why a check and not just the generator
#
# `pixi run third-party` writes the notice. Nothing depended on it, so nothing ever noticed
# when it went stale — and it had been stale in the worst possible way: `ci/third-party.sh`
# was built from `cargo tree`, so the ~246 npm packages and 8 font families that
# `crates/fm-serve/build.rs` bakes into the same binary appeared in no notice at all. MIT,
# Apache-2.0 and the OFL all require their notices to travel with a binary distribution.
#
# The durable shape: **a gate keyed on one manifest is blind to everything the artifact
# carries that is not in it.** `deny.toml` and `cargo tree` are both keyed on `Cargo.lock`.
#
# ## Why the file is committed rather than generated only at release time
#
# `release.yml` drops a generated copy into every archive, which serves a downloader. It does
# not serve someone reading the repository, who would otherwise have to clone and run pixi to
# learn what the binary links. A public repo needs the answer visible.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
committed="$root/THIRD-PARTY.md"

[ -f "$committed" ] || {
    echo "third-party-check: $committed is missing — run 'pixi run third-party'" >&2
    exit 1
}

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

sh "$root/ci/third-party.sh" "$tmp/THIRD-PARTY.md" >/dev/null

if diff -u "$committed" "$tmp/THIRD-PARTY.md" > "$tmp/drift" 2>&1; then
    echo "third-party-check: THIRD-PARTY.md is current"
    exit 0
fi

echo "third-party-check: THIRD-PARTY.md is stale — a dependency changed and the notice did not." >&2
echo >&2
head -60 "$tmp/drift" >&2
echo >&2
echo "  Fix: pixi run third-party   (then commit THIRD-PARTY.md)" >&2
exit 1
