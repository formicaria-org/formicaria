# 2026-07-19 — there is an Android app

**M0's kill criterion passed.** `formicaria.apk` installs, launches, paints, and writes a note
to the device's own private storage. `shots/2026-07-19-android-app-running.png`.

## What exists

`mobile/src-tauri` — a Tauri v2 shell that is thin by construction. **One** `#[tauri::command]`,
taking a command name and a JSON blob, forwarding straight to `fm_app::dispatch`: the same door
`fm-serve` uses. Thirty wrapper functions would have been thirty places to forget one, and the
wire contract already *is* "name plus JSON". That is ruling 1 paying for itself — a second
frontend cost a file, not a fork.

`ui/src/lib/ipc.ts` gains a native branch **ahead of** the `PROD` fork. A Tauri build *is*
`PROD`, so without that ordering the app would `fetch('/api/…')` against a server that does not
exist on the device. One bundle, three backends, most specific first.

## The loop that closed

Tap **+ New** → the scoped command palette → **New note** → Tauri IPC → `fm_app::dispatch` →
`capture` → `FileStore` → a real Markdown file:

```
/data/data/dev.formicaria.notes/vault/notes/01KXWN0G0MG27E3N5V3P9XJDVJ.md
```

Owned by the app's own UID, in app-private storage. Files-as-truth, on a phone, in an installed
app.

**The IPC bridge was proven before that**, incidentally: the "git isn't installed" banner is
`ping` reaching Rust and getting `git: false` back. The capability model degrading correctly, in
a native app, on a platform where git genuinely does not exist.

## Tauri #15671 did not reproduce

Prior research flagged a blank Android WebView on Tauri 2.11.x against Android 14/15 emulators.
We are on **CLI 2.11.4, Android 15, x86_64 emulator** — and it paints. The transport ruling's
fallback (`fm-serve` on device) stays documented but is not needed.

## Four traps, each of which cost a build

1. **`rustup target add`.** Tauri shells out to it; this project has no rustup — Rust and its
   Android std libs come from conda-forge, pinned in `pixi.lock`. Installing a second toolchain
   to satisfy a subcommand would undo the reproducibility that pinning buys. `.android/bin/rustup`
   is a shim that **verifies the target exists and fails loudly if not**, rather than pretending.
2. **Java 25.** conda-forge's default `openjdk` is 25, whose class file version (69) Gradle 8.14
   refuses — in "semantic analysis", with an error naming neither Java nor Gradle. Pinned to
   `21.*`. Note a stale **Gradle daemon** keeps the old JVM: `gradlew --stop` after changing it,
   or the next build fails identically and looks like the pin did nothing.
3. **`node tauri` from the wrong directory.** Tauri's generated Gradle task runs the CLI from
   `src-tauri`, and it records *how it was invoked*. Called by absolute path from `ui/`, it
   generated `node tauri` and Gradle failed with `Cannot find module .../src-tauri/tauri`.
   Fixed by giving `mobile/` its own `package.json` with the CLI beside the shell — the layout
   Tauri assumes — after which it generates `pnpm tauri` and works.
4. **`beforeBuildCommand` and `frontendDist` resolve from different directories** — `mobile/`
   and `mobile/src-tauri/` respectively. Worth pinning down once rather than rediscovering.

## The hole this closed, and one it opened

`configure_paths` sets `FM_CONFIG_DIR` from Tauri's `app_data_dir()`. That is the fix for what
Settings surfaced hours earlier: `config_dir()` returns `None` on Android because nothing sets
that variable, so a phone could not persist a vault list. The **shell** is the right place to
supply it — it is exactly the kind of fact only the platform has.

**Found immediately, unfixed:** the status bar overlaps the toolbar (the clock sits on the
search field). No safe-area inset. `viewport-fit=cover` + `env(safe-area-inset-*)` is the fix,
and it needs a device to judge.

## What is still not done

- **`native-git` has no pull/merge**, so the app cannot sync. It is linked and compiled in, and
  `commit_all`/`clone` work, but the merge half is deliberately absent.
- **arm64 is unbuilt.** Only `--target x86_64` (the emulator) has been built. The owner's phone
  needs `--target aarch64`, which should be a flag change and has not been proven.
- **`open_external` is an honest `Unsupported`** — Android needs an `Intent`, which needs
  `tauri-plugin-opener`. So a PDF cannot be opened yet.
- **Release signing, Play policy, 16 KB alignment** — none of it touched. This is a debug APK.
