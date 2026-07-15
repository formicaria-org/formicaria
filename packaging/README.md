# Launching formicarium

formicarium runs as a local web app: a tiny server (`fm-serve`) builds and serves
the UI and fronts the vault over `http://127.0.0.1:8765`, which it opens in your
default browser.

## Terminal

```sh
pixi run serve          # build + serve; open the printed URL
FM_OPEN=1 pixi run serve # also opens your browser automatically
```

## App-menu icon + Ubuntu dock (GNOME)

The launcher ships its own icon (`formicarium.svg`, an ant — *formica*). Install
the desktop entry, then pin it to the dock.

1. Make the launcher executable (once):
   ```sh
   chmod +x packaging/formicarium.sh
   ```
2. Install the desktop entry so it appears in **Activities → search**:
   ```sh
   cp packaging/formicarium.desktop ~/.local/share/applications/
   update-desktop-database ~/.local/share/applications 2>/dev/null || true
   ```
   The `Exec=` and `Icon=` lines use absolute paths — if you move the repo, edit
   them in the `.desktop` file first (both point at this `packaging/` folder).
3. **Pin it to the panel/dock:** press <kbd>Super</kbd>, type "formicarium",
   right-click the result → **Add to Favorites**. It now stays on the Ubuntu
   dock; click it to launch.

   Prefer the terminal? Add it to the dock declaratively:
   ```sh
   # append formicarium to the current favourites list
   current=$(gsettings get org.gnome.shell favorite-apps)
   gsettings set org.gnome.shell favorite-apps \
     "${current%]*}, 'formicarium.desktop']"
   ```

Clicking the icon runs `packaging/formicarium.sh` → builds + serves + opens the
browser. The server keeps running in the background; stop it with
`pkill -x fm-serve` when you're done.

### Troubleshooting

- **Generic/blank icon.** GNOME caches icons — log out/in, or (X11 only)
  `killall -HUP gnome-shell`. Confirm the entry is valid with
  `desktop-file-validate ~/.local/share/applications/formicarium.desktop`.
- **Launching from a Desktop copy** (not the app menu) needs the file marked
  trusted: `gio set ~/Desktop/formicarium.desktop metadata::trusted true` and
  `chmod +x` it.
- **Nothing opens.** `pixi` may be off the launcher's minimal PATH; the script
  falls back to `~/.pixi/bin/pixi`. Run `pixi run serve` in a terminal once to
  see any build error directly.
