#!/usr/bin/env bash
# Launch formicaria: open it in your browser, starting the local server only if
# it isn't already running. Double-clicked via formicaria.desktop, or run
# directly from a terminal.
#
# Behaviour:
#   - If the server is ALREADY up, just open a fresh browser tab and exit. This
#     is what makes clicking the icon reopen the app every time — it does NOT
#     start a second server (a second one can't bind the port and dies silently,
#     which is why "click again to reopen" used to do nothing).
#   - Otherwise start the pre-built RELEASE binary (target/release/fm-serve),
#     which opens the browser once it is up. No rebuild on launch — update what
#     runs with `pixi run build`. The only auto-build is the first launch (or one
#     after a `cargo clean`), when no binary exists yet.
#
# The server keeps running in the background after you close the tab; stop it
# with `pkill -x fm-serve`.
set -euo pipefail

# The repo root is this script's parent directory (packaging/ lives at the repo
# root), resolved so a double-click from anywhere works.
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO"

ADDR="${FM_ADDR:-127.0.0.1:8765}"
HOST="${ADDR%%:*}"
PORT="${ADDR##*:}"
URL="http://${ADDR}"

# Is a server already listening? Probe the port with bash's /dev/tcp (no curl/nc
# dependency). If so, just (re)open a browser tab and stop here.
if (exec 3<>"/dev/tcp/${HOST}/${PORT}") 2>/dev/null; then
    xdg-open "$URL" >/dev/null 2>&1 || true
    exit 0
fi

# pixi may not be on a desktop launcher's minimal PATH — find it explicitly.
PIXI="$(command -v pixi || true)"
[ -z "$PIXI" ] && PIXI="$HOME/.pixi/bin/pixi"

# First run / after a clean: no release artifacts yet → build them once.
if [ ! -x "$REPO/target/release/fm-serve" ] || [ ! -d "$REPO/ui/dist" ]; then
    "$PIXI" run build
fi

export FM_OPEN=1          # open the browser once the server is up
export FM_AUTO_SHUTDOWN=1 # ...and quit the server when the tab is closed
# Run the prebuilt release binary directly, with the pixi env's `bin` on PATH.
#
# **Skipping `pixi run` is the whole startup optimisation**, and it was measured rather than
# assumed. `pixi run` costs **~1.4s whenever its activation cache is cold** — which is exactly the
# double-click case — against 0.07s warm. `fm-serve` itself binds in 35–43ms with the real vaults.
# So on the launch that matters most, pixi was ~97% of the wait.
#
# `--frozen` does **not** avoid it: that was measured too, and the first attempt to prove it was
# wrong because the two forms were alternated and plain always ran first, warming the cache for
# frozen. Cold, both pay it.
#
# **What activation actually provides here is PATH, and nothing else that matters.** Diffing
# `pixi run env` against a plain one: no `LD_LIBRARY_PATH` is set at all, and the only runtime
# names are `PATH` and `CONDA_PREFIX`. `git` comes from `/usr/bin`; `pdftotext`, `vipsthumbnail`
# and `restic` live in the env's `bin` and were each run with PATH alone, resolving every shared
# library (conda binaries carry their own RPATH). The rest of what activation sets is the conda
# *build* toolchain — `CC`, `CFLAGS`, `CMAKE_ARGS` — which a running server has no use for.
#
# The fallback matters: with no env installed yet there is nothing to put on PATH, so hand over to
# pixi, which will build it. That is the first-run path, where 1.4s is irrelevant next to a build.
ENV_BIN="$REPO/.pixi/envs/default/bin"
if [ -x "$REPO/target/release/fm-serve" ] && [ -d "$ENV_BIN" ]; then
    export PATH="$ENV_BIN:$PATH"
    exec "$REPO/target/release/fm-serve"
fi
exec "$PIXI" run app
