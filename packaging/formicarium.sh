#!/usr/bin/env bash
# Launch formicarium: build the UI, serve the vault, and open the default browser.
# Double-clicked via formicarium.desktop, or run directly from a terminal.
set -euo pipefail

# The repo root is this script's parent directory's parent (packaging/ lives at
# the repo root), resolved so a double-click from anywhere works.
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO"

# pixi may not be on a desktop launcher's minimal PATH — find it explicitly.
PIXI="$(command -v pixi || true)"
[ -z "$PIXI" ] && PIXI="$HOME/.pixi/bin/pixi"

export FM_OPEN=1
exec "$PIXI" run serve
