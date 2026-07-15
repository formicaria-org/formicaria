#!/usr/bin/env bash
# Launch formicarium: run the latest RELEASE build and open the default browser.
# Double-clicked via formicarium.desktop, or run directly from a terminal.
#
# This runs the pre-built optimized binary (target/release/fm-serve) WITHOUT
# rebuilding, so launching is instant. It always reflects the last build you
# made — update what it runs by rebuilding explicitly:
#
#     pixi run build
#
# The only time it builds for you is the very first launch (or one right after a
# `cargo clean`), when there is no binary yet — otherwise the icon would be dead.
set -euo pipefail

# The repo root is this script's parent directory (packaging/ lives at the repo
# root), resolved so a double-click from anywhere works.
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO"

# pixi may not be on a desktop launcher's minimal PATH — find it explicitly.
PIXI="$(command -v pixi || true)"
[ -z "$PIXI" ] && PIXI="$HOME/.pixi/bin/pixi"

# First run / after a clean: no release artifacts yet → build them once.
if [ ! -x "$REPO/target/release/fm-serve" ] || [ ! -d "$REPO/ui/dist" ]; then
    "$PIXI" run build
fi

export FM_OPEN=1
# Run the prebuilt release binary in the pixi env (no rebuild). `pixi run app`
# activates the env so the ingest subprocesses (pdftotext, vipsthumbnail) are
# found on PATH.
exec "$PIXI" run app
