#!/bin/sh
# **`pixi run privacy-coverage`** (`privacy-plan-2026-11-19.md` 1.4): the lists fetched as published
# today into `target/privacy-lists/`, and what each set refuses of the corpus (`crates/fm-block/tests/data/`).
set -eu
d=target/privacy-lists
mkdir -p "$d"
get() { curl -sSfL --max-time 60 "$2" -o "$d/$1.txt"; }
get easyprivacy https://easylist.to/easylist/easyprivacy.txt
get easylist https://easylist.to/easylist/easylist.txt
for k in filters privacy unbreak quick-fixes badware; do
  get "ubo-$k" "https://ublockorigin.github.io/uAssetsCDN/filters/$k.min.txt"
done
get pgl "https://pgl.yoyo.org/adservers/serverlist.php?hostformat=adblockplus&mimetype=plaintext"
get ddg-apps https://raw.githubusercontent.com/duckduckgo/tracker-blocklists/main/app/android-tds.json
cargo run -q --release -p fm-block --example coverage -- "$d"
