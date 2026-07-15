#!/usr/bin/env bash
# Install (or refresh) the formicarium launcher for GNOME / freedesktop desktops.
#
# Why a script: GNOME Shell reliably resolves an icon only when it is a NAMED icon
# installed in an icon theme (hicolor). An absolute path to an SVG in the desktop
# entry's `Icon=` line often shows a generic ("yellow") fallback instead. So we
# install two things:
#   - the icon, by name, at  ~/.local/share/icons/hicolor/scalable/apps/formicarium.svg
#   - the desktop entry       ~/.local/share/applications/formicarium.desktop  (Icon=formicarium)
# then refresh the caches. Run this once.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APPS="$HOME/.local/share/applications"
ICONDIR="$HOME/.local/share/icons/hicolor/scalable/apps"
mkdir -p "$APPS" "$ICONDIR"

# 1. Themed icon, referenced by NAME (Icon=formicarium) from the .desktop entry.
cp "$HERE/formicarium.svg" "$ICONDIR/formicarium.svg"

# 2. Desktop entry + make the launcher itself executable.
cp "$HERE/formicarium.desktop" "$APPS/formicarium.desktop"
chmod +x "$HERE/formicarium.sh"

# 3. Refresh the freedesktop caches (best-effort; the named icon resolves even
#    without a built cache, but this makes the entry show up immediately).
update-desktop-database "$APPS" 2>/dev/null || true
gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true

echo "Installed formicarium launcher + icon."
echo
echo "GNOME caches app icons in the running shell. To see the new icon:"
if [ "${XDG_SESSION_TYPE:-}" = "wayland" ]; then
    echo "  You are on Wayland — log out and back in (the shell can't hot-reload)."
else
    echo "  On X11: press Alt+F2, type 'r', Enter — or log out and back in."
fi
