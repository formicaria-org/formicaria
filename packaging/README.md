# Launching formicarium

formicarium runs as a local web app: a tiny server (`fm-serve`) builds and serves
the UI and fronts the vault over `http://127.0.0.1:8765`, which it opens in your
default browser.

## Terminal

```sh
pixi run serve          # build + serve; open the printed URL
FM_OPEN=1 pixi run serve # also opens your browser automatically
```

## Double-click icon (Linux)

1. Make the launcher executable (once):
   ```sh
   chmod +x packaging/formicarium.sh
   ```
2. Install the desktop entry so it appears in your app menu / is double-clickable:
   ```sh
   cp packaging/formicarium.desktop ~/.local/share/applications/
   update-desktop-database ~/.local/share/applications 2>/dev/null || true
   ```
   (If you moved the repo, edit the `Exec=` path in the `.desktop` file first.)

Double-clicking runs `packaging/formicarium.sh`, which builds + serves and opens
the browser. The server keeps running in the background; stop it with
`pkill -x fm-serve` when you're done.
