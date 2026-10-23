#!/bin/sh
# Serve an imported vault and check, in a real browser, that its notes show what they link to.
#
# Layer 3 of docs/context/logseq-import-plan-2026-10-14.md. Run `pixi run import-corpus` first: it
# leaves the imported vault in target/import-corpus/work/vault, which is what this serves. Nothing
# here touches a user's real vaults, config or port 8765; the vault is a throwaway the Rust test
# re-creates on every run.
#
#   pixi run import-render            # FM_RENDER_SAMPLE=60 notes without attachments, plus all with
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"

vault=${FM_RENDER_VAULT:-target/import-corpus/work/vault}
port=${FM_RENDER_PORT:-8792}
[ -d "$vault" ] || {
    echo "import-render: no imported vault at $vault — run \`pixi run import-corpus\` first" >&2
    exit 1
}
command -v chromium >/dev/null 2>&1 || {
    echo "import-render: chromium is not on PATH — it is the browser this drives" >&2
    exit 1
}
[ -x target/release/fm-serve ] || {
    echo "import-render: building first (pixi run build)…"
    pixi run build >/dev/null
}

work=$(mktemp -d)
cleanup() {
    [ -n "${serve_pid:-}" ] && kill "$serve_pid" 2>/dev/null || true
    rm -rf "$work"
}
trap cleanup EXIT INT TERM

printf '{ "vaults": [ { "name": "imported", "path": "%s" } ] }\n' "$(cd "$vault" && pwd)" \
    > "$work/vaults.json"
FM_VAULTS="$work/vaults.json" FM_ADDR="127.0.0.1:$port" FM_OPEN=0 FM_AUTO_SHUTDOWN=0 \
    ./target/release/fm-serve > "$work/serve.log" 2>&1 &
serve_pid=$!

i=0
until curl -fs -o /dev/null "http://127.0.0.1:$port/"; do
    i=$((i + 1))
    [ "$i" -lt 60 ] || { echo "import-render: fm-serve did not come up; see $work/serve.log" >&2; exit 1; }
    sleep 0.5
done

# A separate CDP port and profile from `shots`, so the two can never fight over one browser.
FM_SHOT_URL="http://127.0.0.1:$port/" FM_SHOT_CDP_PORT=${FM_RENDER_CDP_PORT:-9223} \
    FM_SHOT_PROFILE="~/snap/chromium/common/fm-import-render" \
    PYTHONPATH=ci python3 ci/import-render.py
