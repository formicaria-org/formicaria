# 2026-07-19 — formicaria's core, running on an actual phone

**The first time any formicaria code has executed on a phone.** Not a browser pointed at a
laptop — the compiled Rust core, on the device's own CPU, writing the device's own disk.

## What was done

`fm-cli` cross-compiled for `aarch64-linux-android` (a real PIE executable, 49 MB debug),
pushed with `adb push` to `/data/local/tmp`, and run over `adb shell`.

**Why that works without a shell, an APK, or the SDK:** `adb shell` runs as the *shell* user,
outside the app sandbox, so the W^X restriction that stops an app exec'ing a binary from its own
data directory does not apply. It is not how the app will ever ship — it is the cheapest possible
way to answer "does the core actually work on ARM", months before there is anything to install.

## What it proved

| Question | Answer |
|---|---|
| Does the core run on ARM? | **Yes** — `fm --help` printed the real command surface |
| Does capture write files-as-truth? | **Yes** — real `<ulid>.md` with correct frontmatter on the phone's disk |
| Does SQLite work on ARM? | **Yes** — `list` round-tripped through the index |
| **Does FTS5 work on ARM?** | **Yes** — `search algorithm` returned exactly the right note |
| Is there a `git` binary on the device? | **No** — `git: inaccessible or not found` |
| Does it degrade gracefully without git? | **Yes** — capture/list/search all fine, no `.git` created |

The last two are the important ones, and they are why this was worth doing.

**The premise under the whole libgit2 decision is now verified rather than assumed.** Every
document in `docs/context/` asserts "a phone has no `git` binary to shell out to" — sourced from
platform documentation. It is now sourced from *this phone*, which is a different kind of fact.

**And the capability model holds in the one place it was only ever theory.** `git::available()`
returns false, `commit_all` is gated on it, and the result is a working notebook that simply has
no history — exactly what `decisions.md` describes as "a compliant degraded notebook that reports
`git: false`". That path had never been executed on a machine where git genuinely does not exist;
every previous test simulated absence on a machine that had it.

## What it does not prove

- **Nothing about the UI.** This is the CLI. The step-0 finding — the GUI is not good on small
  screens — is untouched by any of this.
- **Nothing about the app being installable.** There is still no shell, no APK, no SDK. A binary
  in `/data/local/tmp` is a test fixture, not a product; an *app* cannot exec from its own data
  directory, which is the whole reason the port needs an in-process git rather than a bundled one.
- **Nothing about `native-git` working.** It *compiles* for the target (libgit2, libcrypto and
  libssl are all AArch64, asserted via `readelf`), but no code calls it — `git.rs` still shells
  out, and on this device that means it does nothing at all.

## Left on the device

`/data/local/tmp/fm` and `/data/local/tmp/vault` (three test notes). Harmless, and useful for
re-running this. `adb shell rm -rf /data/local/tmp/fm /data/local/tmp/vault` clears it.

## Next

`pixi run android-check` gates the build; this session gates the *behaviour*, and there is no
task for it because it needs a device attached. The genuine remaining gaps, in order: the SDK and
an emulator (so something can be *installed* rather than pushed), a shell, and a `native-git`
implementation graded against the step-2 differential harness.
