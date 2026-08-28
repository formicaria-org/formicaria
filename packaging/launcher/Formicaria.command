#!/bin/sh
# Start formicaria on macOS from an unpacked release archive.
#
# **`.command` rather than a plain script** because that is the extension Finder will run on a
# double-click. A Terminal window appears alongside the app and must stay open — that is a
# property of `.command`, not something this script can avoid, and the README says so rather than
# pretending otherwise.
#
# **The first launch needs right-click -> Open, not a double-click.** These binaries are not
# signed or notarised, so Gatekeeper refuses a plain double-click. Right-click -> Open offers an
# "Open anyway" button; after once, double-click works. The alternative is a paid Apple developer
# account, which is why the README documents the click instead of shipping a signature.
#
# `dirname $0` is load-bearing here: Finder launches with the working directory set to the user's
# home, so anything relative would resolve against the wrong folder entirely.
set -eu

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

# See formicaria.sh — both vault variables are set together, deliberately.
FM_VAULT="$here/vault"
FM_VAULTS="$here/vaults.json"
FM_OPEN=1
FM_AUTO_SHUTDOWN=1
export FM_VAULT FM_VAULTS FM_OPEN FM_AUTO_SHUTDOWN

mkdir -p "$FM_VAULT"

exec "$here/fm-serve"
