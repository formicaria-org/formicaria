#!/bin/sh
# Copy the food pack just built to where this machine's formicaria looks for one.
#
# For trying a pack before it is published anywhere: the app's own download needs an address, and
# until a pack has one this is how a maintainer gets it onto a desktop. A phone has no such door.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
src="$root/target/packs/usda-foods.sqlite"
[ -f "$src" ] || { echo "pack-install: build it first (pixi run pack-foods)" >&2; exit 1; }
dest="${FM_PACKS_DIR:-${XDG_CONFIG_HOME:-$HOME/.config}/formicaria/tools/packs}"
mkdir -p "$dest"
cp "$src" "$dest/usda-foods.sqlite"
echo "installed: $dest/usda-foods.sqlite"
