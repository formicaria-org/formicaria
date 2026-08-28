#!/bin/sh
# Start formicaria from an unpacked release archive. Double-click this.
#
# **Not `packaging/formicaria.sh`.** That one is the developer launcher: it assumes a git
# checkout (`REPO=`, `target/release/`, `.pixi/envs/`) and falls back to `pixi run build`, which
# from a USB stick would try to compile the whole workspace. This one assumes nothing but the
# files beside it.
#
# Everything is resolved from **this script's own directory**, never the working directory. That
# is the exact hazard behind the 2026-07-17 ruling that removed the relative `FM_VAULT="vault"`
# default: a launcher started from elsewhere silently created an empty vault while the real notes
# appeared to have vanished. `FM_VAULT` set *explicitly* stayed valid, and this is that path.
set -eu

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

# The app IS this folder. Copy it to a USB stick and the notes travel with it.
#
# **Both variables, always.** `FM_VAULT` alone is not enough: with no `vaults.json` beside it,
# creating a second vault makes the list start winning, `FM_VAULT` is ignored, and vault #1
# disappears on the next start. Setting both keeps the whole configuration inside this folder,
# and off the machine that happens to be running it.
FM_VAULT="$here/vault"
FM_VAULTS="$here/vaults.json"
FM_OPEN=1            # open the browser; without this the user gets a terminal and no app
FM_AUTO_SHUTDOWN=1   # closing the tab stops the server, instead of leaving 8765 held forever
export FM_VAULT FM_VAULTS FM_OPEN FM_AUTO_SHUTDOWN

mkdir -p "$FM_VAULT"

exec "$here/program/fm-serve"
