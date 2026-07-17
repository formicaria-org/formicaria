#!/usr/bin/env bash
# Install (or refresh) the formicaria launcher for GNOME / freedesktop desktops.
#
# Why a script: GNOME Shell reliably resolves an icon only when it is a NAMED icon
# installed in an icon theme (hicolor). An absolute path to an SVG in the desktop
# entry's `Icon=` line often shows a generic ("yellow") fallback instead. So we
# install two things:
#   - the icon, by name, at  ~/.local/share/icons/hicolor/scalable/apps/formicaria.svg
#   - the desktop entry       ~/.local/share/applications/formicaria.desktop  (Icon=formicaria)
# then refresh the caches. Re-run it after moving the repo — see below.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APPS="$HOME/.local/share/applications"
ICONDIR="$HOME/.local/share/icons/hicolor/scalable/apps"
mkdir -p "$APPS" "$ICONDIR"

# 1. Themed icon, referenced by NAME (Icon=formicaria) from the .desktop entry.
cp "$HERE/formicaria.svg" "$ICONDIR/formicaria.svg"

# 2. Desktop entry, GENERATED with this checkout's real path.
#
# `.desktop` has no notion of a relative Exec — it must be absolute. So the entry
# cannot be a static file in the repo: one committed here would carry whatever
# absolute path its author happened to have, and every clone would get a launcher
# pointing into a stranger's home directory. (It did, and a rename once rewrote the
# path to something that didn't exist — the file looked right and launched nothing.)
# Generating it means the path is always *this* checkout, and moving the repo is a
# re-run of this script rather than a silent breakage.
cat > "$APPS/formicaria.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=formicaria
GenericName=Research notebook
Comment=Local-first notes — opens in your browser
Exec=$HERE/formicaria.sh
Icon=formicaria
Terminal=false
Categories=Office;
Keywords=notes;pkm;notebook;
EOF
chmod +x "$HERE/formicaria.sh"

# 3. Refresh the freedesktop caches (best-effort; the named icon resolves even
#    without a built cache, but this makes the entry show up immediately).
update-desktop-database "$APPS" 2>/dev/null || true
gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true

echo "Installed formicaria launcher + icon."
echo
echo "GNOME caches app icons in the running shell. To see the new icon:"
if [ "${XDG_SESSION_TYPE:-}" = "wayland" ]; then
    echo "  You are on Wayland — log out and back in (the shell can't hot-reload)."
else
    echo "  On X11: press Alt+F2, type 'r', Enter — or log out and back in."
fi
