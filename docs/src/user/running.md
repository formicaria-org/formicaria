# Running formicaria

formicaria runs in your browser, served by a small local process. Everything is
pinned through [pixi](https://pixi.sh), so the toolchain and subprocess tools
(`pdftotext`, `vipsthumbnail`, `restic`) are reproducible.

## From a terminal

```sh
pixi run serve
```

This builds the UI, starts the server on `http://127.0.0.1:8765`, and prints the
URL. Open it in your browser. To have it open the browser for you:

```sh
FM_OPEN=1 pixi run serve
```

There is no default vault: on first run the app asks you to create one, and remembers it in
the vault list. To pin one explicitly (this is what `pixi run serve` does for `./vault`):

```sh
FM_VAULT=~/notes pixi run serve
```

## As a double-click icon (Linux)

A desktop launcher is provided in `packaging/`:

```sh
chmod +x packaging/formicaria.sh
cp packaging/formicaria.desktop ~/.local/share/applications/
```

Double-clicking it runs the launcher (which sets `FM_OPEN=1`) and opens your
browser. The server keeps running in the background; stop it with
`pkill -x fm-serve`. See `packaging/README.md` for details.

## Environment variables

| Variable          | Default            | Meaning                                   |
|-------------------|--------------------|-------------------------------------------|
| `FM_VAULT`        | *(none — asked on first run)* | The vault directory (notes + index).      |
| `FM_ADDR`         | `127.0.0.1:8765`   | Address the server binds.                 |
| `FM_OPEN`         | *(unset)*          | If set, opens the browser on startup.     |
| `FM_RESTIC_REPO`  | *(unset)*          | restic repository for **Back up**.        |
| `RESTIC_PASSWORD` | *(unset)*          | Password for that restic repository.      |
