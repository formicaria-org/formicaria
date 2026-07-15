# 2026-07-15 — Launcher UX: release icon, themed icon, reopen, auto-shutdown

**Outcome:** made the desktop icon behave like a real app launcher. Four fixes,
verified (`pixi run ci` exit 0; live server tests of reopen + auto-shutdown).
Built + committed alongside the build-tasks work.

## What we implemented

- **Icon runs the prebuilt release, no rebuild per click.** The launcher ran
  `pixi run serve` — a *debug* build recompiled on every click. New `pixi run app`
  task runs `target/release/fm-serve` in the activated env (so ingest's
  `pdftotext`/`vipsthumbnail` stay on PATH), no rebuild; the launcher execs that.
  Update what the icon runs with `pixi run build`. It only auto-builds once, when
  the binary is missing (first launch / after `cargo clean`).
- **Fixed the "yellow"/generic icon.** GNOME Shell reliably resolves a *named,
  themed* icon, not an absolute-path SVG in `Icon=`. Changed the `.desktop` to
  `Icon=formicarium` and added `packaging/install.sh`, which installs the SVG to
  `~/.local/share/icons/hicolor/scalable/apps/formicarium.svg` + the desktop entry
  and refreshes caches. On **Wayland** (Ubuntu default) the shell can't hot-reload
  its icon cache — you must **log out/in** after install.
- **Fixed "can't reopen after closing the tab."** The old launcher started a
  second `fm-serve` on every click; with one already holding the port, the second
  died silently on bind. The launcher now probes the port (bash `/dev/tcp`, no
  curl/nc dep) and, if a server is up, just `xdg-open`s a fresh tab.
- **Closing the tab closes the app (auto-shutdown).** The user found a lingering
  background server counter-intuitive and against the lightweight goal. The UI now
  heartbeats `POST /api/ping` every 3s (only in the `PROD` served build; the mock
  is a no-op). `fm-serve` tracks `last_seen`/`connected` and, when
  `FM_AUTO_SHUTDOWN` is set (the launcher sets it, `pixi run serve` doesn't), runs
  a watchdog that `process::exit(0)`s after a **10s** idle window (60s grace before
  first contact). The window is wider than a page reload's gap, so refreshing
  doesn't kill it; closing the tab does, ~10-12s later.

## Decisions

- **Auto-shutdown is opt-in via `FM_AUTO_SHUTDOWN`**, not always-on — a terminal
  `pixi run serve` (dev loop, browser opened manually/late) must not self-exit.
- **Heartbeat timeout, not a `beforeunload` "quit" beacon.** `beforeunload`/
  `pagehide` also fire on reload/navigation, so a quit-beacon would kill the
  server on every refresh. The idle-timeout survives reloads by construction.
- **Icon by theme name, absolute path only for `Exec=`.** Named icons are what
  GNOME resolves; the launcher path legitimately needs to be absolute.

## Notes / gotchas

- **Sizes are dev-only.** The repo folder hit 14 GB (target 9.8 GB debug cache +
  .pixi 3.3 GB toolchain + node_modules); `cargo clean` + removing the orphaned
  `.pixi/envs/gui` (leftover from the removed Tauri window) took it to ~2.8 GB.
  The shipped app is unchanged: 2.7 MB binary + 4.5 MB `ui/dist`. `.pixi/envs/
  default` (2.5 GB) is mostly Rust+LLVM + a C compiler (rusqlite builds SQLite
  from C) + Node — normal, and never ships.
- Wayland has no `Alt+F2 → r`; icon-cache refresh needs a real log out/in.
