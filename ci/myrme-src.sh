#!/bin/sh
# Print the root of the Myrme source this build links — the checkout Cargo made of the pinned tag
# (or a developer's local `[patch]`). **The one way formicaria reads Myrme's files** (2026-10-19):
# the guards in `ci/checks.sh`, the manual's assistant chapter (`ci/myrme-manual.sh`) and the
# phone's runtime pins (`ci/android-stage-*.sh`) all come from here, so formicaria keeps no copy.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
src=$(cd "$root" && cargo metadata --format-version 1 2>/dev/null \
    | grep -o '"manifest_path":"[^"]*/fm-agent-run/Cargo.toml"' | head -1 \
    | sed 's/^"manifest_path":"//; s|/crates/fm-agent-run/Cargo.toml"$||')
[ -n "$src" ] && [ -d "$src" ] || { echo "myrme-src: cannot find Myrme's source through \`cargo metadata\`" >&2; exit 1; }
echo "$src"
