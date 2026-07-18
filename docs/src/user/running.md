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
the vault list (`~/.config/formicaria/vaults.json`). To pin one explicitly:

```sh
FM_VAULT=~/notes pixi run serve
```

**`FM_VAULT` only applies while you have no saved vault list** — once `vaults.json` exists it
wins, because it is the thing that can hold more than one vault. That is why `pixi run serve`
sets `FM_VAULTS=.dev-vaults.json` as well: without it, creating a vault in the app would write
your *real* vault list, and from then on the dev loop would quietly read that instead of
`./vault`.

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
| `FM_VAULT`        | *(none — asked on first run)* | A single vault directory. Ignored once a vault list exists. |
| `FM_VAULTS`       | `~/.config/formicaria/vaults.json` | The vault list file. **Takes precedence over `FM_VAULT`.** |
| `FM_UI_DIST`      | *(unset — serves the UI baked into the binary)* | Read the UI from a directory instead. The dev loop points it at `ui/dist`. |
| `FM_AUTO_SHUTDOWN`| *(unset)*          | If set, the server exits once the last browser tab stops beating — so closing the tab closes the app. The desktop launcher sets it; `pixi run serve` does not. |
| `FM_ADDR`         | `127.0.0.1:8765`   | Address the server binds.                 |
| `FM_OPEN`         | *(unset)*          | If set, opens the browser on startup.     |
| `FM_RESTIC_REPO`  | *(unset)*          | restic repository for **Back up**.        |
| `RESTIC_PASSWORD` | *(unset)*          | Password for that restic repository.      |
