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
# Closing the browser tab shuts the server down (FM_AUTO_SHUTDOWN, set below), so
# a launched-by-icon server does not linger — double-click to start, close to stop.
# (A dev server from `pixi run serve` does NOT set that flag and will keep running.)
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
# dependency). If so, REUSE it — just (re)open a browser tab — **unless it is stale**:
# started before the current binaries, UI or model config were last changed (a
# `pixi run build`, or editing `models.toml` to switch the model). Reusing a stale
# server is exactly how a double-click kept serving the OLD model after a change.
# When stale, stop it (the agent self-stops with it) and fall through to a fresh
# start, so a click after any change always runs the current version.
if (exec 3<>"/dev/tcp/${HOST}/${PORT}") 2>/dev/null; then
    pid="$(pgrep -x fm-serve | head -1)"
    started="$(date -d "$(ps -o lstart= -p "$pid" 2>/dev/null)" +%s 2>/dev/null || echo 0)"
    stale=0
    for f in target/release/fm-serve target/release/agent-serve agents/models.toml ui/dist; do
        [ -e "$f" ] && [ "$(stat -c %Y "$f" 2>/dev/null || echo 0)" -gt "$started" ] && stale=1
    done
    if [ "$stale" -eq 0 ]; then
        xdg-open "$URL" >/dev/null 2>&1 || true   # current — reuse; the browser focuses an open tab
        exit 0
    fi
    echo "formicaria: a newer build or model config is present — restarting the server" >&2
    # Stop the WHOLE stack, not just fm-serve. If only fm-serve is killed and a new one is started on
    # the same port, the old `agent-serve` — which watches that port for liveness — finds the NEW
    # fm-serve there before it notices its own parent died, gets "adopted", and never shuts down,
    # keeping the model (and its GPU VRAM) pinned forever. So take the model + agent down too.
    pkill -x fm-serve 2>/dev/null || true
    pkill -x agent-serve 2>/dev/null || true
    pkill -x llama-server 2>/dev/null || true
    # Wait for the port to free before a fresh start.
    for _ in $(seq 1 12); do (exec 3<>"/dev/tcp/${HOST}/${PORT}") 2>/dev/null || break; sleep 1; done
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

# **Which program: the one built here, or a release installed over it from Settings.**
#
# A copy built from source can install the published release the way the phone's test build can
# (`decisions.md`, 2026-10-11). Its program lives in the build tree, which cannot be renamed aside,
# so the app installs into a folder of its own, `target/app/`, laid out like a downloaded copy:
# `program/`, and `.fm-backup-*` for the way back. This launcher says where that is (`FM_APP_DIR`)
# and that it can rescue a bad install (`FM_LAUNCHER=2`); without both the app refuses to install.
#
# Three choices, in this order:
#   1. **Built since the install** (`target/release/fm-serve` is newer than the stamp), or nothing
#      installed: the build made here runs, and what was installed is removed. Building is how you
#      come back to your own code.
#   2. **A release is installed and starts**: it runs. Each start is counted and the app clears the
#      count once it has been serving for a moment, exactly as in `launcher/Start formicaria.sh`.
#   3. **It is missing or failed three starts**: the program it replaced is put back; and if there
#      is nothing to put back, the build made here runs. That one is always there.
export FM_LAUNCHER=2
export FM_APP_DIR="$REPO/target/app"
APP="$FM_APP_DIR"
BUILT="$REPO/target/release/fm-serve"
RUN="$BUILT"
fm_restore() {
    for b in "$APP"/.fm-backup-*; do
        if [ -x "$b/fm-serve" ]; then
            rm -rf "$APP/program"
            mv "$b" "$APP/program"
            rm -f "$APP/.fm-attempts"
            return 0
        fi
    done
    return 1
}
if [ -e "$APP" ]; then
    if [ ! -f "$APP/.fm-installed" ] || [ "$BUILT" -nt "$APP/.fm-installed" ]; then
        rm -rf "$APP"
    else
        attempts="$(cat "$APP/.fm-attempts" 2>/dev/null || echo 0)"
        # Anything that is not a plain number counts as none: a corrupted counter rolls nobody back.
        case "$attempts" in '' | *[!0-9]*) attempts=0 ;; esac
        if [ ! -x "$APP/program/fm-serve" ] || [ "$attempts" -ge 3 ]; then
            if fm_restore; then
                echo "formicaria: putting the previous version back." >&2
                attempts=0
            else
                echo "formicaria: the installed release will not start; running the build made here." >&2
                rm -rf "$APP"
            fi
        fi
        if [ -x "$APP/program/fm-serve" ]; then
            echo $((attempts + 1)) > "$APP/.fm-attempts" 2>/dev/null || true
            RUN="$APP/program/fm-serve"
        fi
    fi
fi

if [ -x "$RUN" ] && [ -d "$ENV_BIN" ]; then
    export PATH="$ENV_BIN:$PATH"
    exec "$RUN"
fi
exec "$PIXI" run app
