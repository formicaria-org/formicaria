# 2026-07-19 — the whole product, rendering on Android

**The UI now runs on Android against an on-device backend**, and the owner's coarse step-0
verdict — *"the current GUI is not good for small screens"* — is a specific defect list.

Screenshots: `docs/context/shots/2026-07-19-android-board.png` (and `-scrolled.png`).

## How, and why it needed no shell

`fm-serve` and `fm-cli` cross-compiled for `x86_64-linux-android`, pushed to the emulator's
`/data/local/tmp`, and **the server run on the device**. Notes captured on-device with `fm`,
then the emulator's own browser pointed at `http://127.0.0.1:8765` — server and browser both
on Android, nothing on the laptop but `adb`.

This is the **ruling-2 fallback transport** (`fm-serve` on device) exercised for real, months
before the Tauri bridge exists. It is not the product — a binary in `/data/local/tmp` is a test
fixture and an *app* could not exec it — but it renders the same bytes the shell will.

Emulator: Android 15, x86_64, headless. `screencap` works with no window, which is what makes
the UI inspectable from a terminal at all. `pixi run android-avd` / `android-emu` make it
repeatable.

## What works

The whole thing. Header, search, New note/New board, the view tabs (Board, Agenda, Timeline,
Search, Activity), the pane workspace, the Board renderer with both notes, status grouping,
per-card actions, the vault chip, the theme toggle, Back up.

**And the capability model degrades correctly, visibly, on a device where git genuinely does not
exist:** the banner reads *"git isn't installed — your notes are saved as files, but not
versioned. Install git for history, backup and sharing."* That is `git::available()` returning
false end-to-end, into copy a user can act on. Previously only ever theory.

## The defect list — the thing that was missing

Ranked by how much they cost on a phone:

1. **Chrome eats ~40% of the viewport before content.** Four stacked rows: title+search+New
   note, New board, the view-tab row, then cols/theme/Back up. On a 2400px-tall screen the
   board starts below the halfway mark. This is the finding — the rest are details.
2. **`cols 2` on a phone.** The board defaults to two columns at a width that fits one. The
   control is right there in the toolbar, which is itself part of problem 1.
3. **The git banner is permanent on mobile**, because git will *never* be installed on Android.
   Correct copy on the desktop, but on a phone it is un-actionable advice occupying a band
   forever — and it will be wrong outright once `native-git` ships, since history will work
   with no `git` binary anywhere. Needs to key on the capability, not the binary.
4. **The view-tab row barely fits** five tabs and has no overflow behaviour; a sixth wraps or
   clips.
5. **Pane chrome is desktop-shaped** — the focus border, the per-pane `×`, the drag handle and
   the two selector inputs are all mouse-sized furniture on a surface with one pane.

Not yet assessed, and needing a real device rather than an emulator: **Board drag versus
scroll-snap**, the tap→move menu, thumb reach, and a `<video>` seeking mid-file.

## What this does not mean

Still no app. No shell, no APK, nothing installable — an app cannot exec a binary from its own
data directory, which is exactly why the port needs in-process git rather than a bundled one.
`native-git` exists and is graded but nothing calls it. The Tauri bridge, and with it the M0
"does the WebView paint" kill criterion, is untouched: this rendered in **Chrome**, not in the
System WebView a Tauri app would use.

## A bug found on the way

`fm-serve` did not compile for Android at all: `open_native` had `#[cfg]` arms for linux, macos
and windows and no fallback, so `cmd` was never bound. Same shape of hole as
`fm_app::vaults::config_dir`. Fixed with an honest `Unsupported` — handing a file to its owner
on Android means an `Intent`, which needs the JVM, which is precisely why `open_external` is a
`Host` trait method rather than a `#[cfg]` ladder.


## First pass at the defects — measured, not guessed

Fixed and verified on the emulator (`shots/2026-07-19-android-board.png` before,
`-board-after.png` after):

- **The `cols` control is hidden below 40rem.** It was rendered, said "2", and did nothing —
  `.workspace` is already forced to one column at that width. A control that lies about the
  state is worse than one that is absent.
- **The view chips are one horizontally-scrolling strip** rather than a wrapping block. Costs
  one row instead of two, and stops being a cliff the moment a sixth saved view exists.
  `touch-action: pan-x` so a vertical drag still scrolls the page.
- **The wordmark is hidden, Back up is icon-only, the separator is gone.**
- **The search input drops 9rem → 6rem.** Measured: row one came to ~423px against a 411px
  viewport — over by about a dozen pixels, which is the entire reason New board sat alone on a
  line. Giving up 3rem closes the row.

**Result: four toolbar rows became three.** Toolbar height ~445 → ~335 display-px, about a 25%
reduction; the board starts ~130px higher on a 2400px screen.

**Honest about what is left.** The third row still holds only the theme toggle and Back up,
right-aligned against an empty left half — that is the next obvious row to reclaim, and it
needs a markup change rather than CSS, so it is a design call rather than a tweak. And the
original "~40% before content" figure was right but worth decomposing: roughly 22% of it is our
toolbar, the rest is the once-per-session git banner (dismissible) plus Chrome's own URL bar,
which a real shell would not have.

## A deployment trap worth knowing

`adb push <dir> <existing-dir>` **nests** rather than replaces — the second push landed in
`dist/dist/`, so the device kept serving the previous bundle while the local build looked
correct. Two CSS changes appeared to do nothing before this was spotted; the tell was comparing
the hashed asset name in the *served* `index.html` against the one just built. `rm -rf` the
target first.


## The toolbar redesign (owner's design, same day)

*"Search, a plus for new atoms, a plus for view, more cmds in a dedicated cmd palette, the
vault filter explicit but not taking much space, and a settings that shows the config the user
is operating with."* Applied to **both** platforms, not a mobile special case.

**Four toolbar rows became one** (`shots/2026-07-19-android-toolbar-one-row.png`): search,
`+ New`, `+ View`, the vault/contributor filters, the palette, settings. Content now starts at
~24% of the screen instead of ~41%.

**The two "+" buttons open the command palette pre-filtered** — `New` and `Open` — rather than
a dropdown. Three reasons, and the first is the one that decided it:
1. **This codebase has deliberately never had a dropdown.** `Pane.svelte` documents its view
   picker as *"a rotator, not a dropdown"*, and there is no popover primitive anywhere.
2. **`.topbar` is a scroll container** (`overflow-x:auto; overflow-y:hidden`), so an anchored
   menu would be clipped vertically and scroll away horizontally. A dropdown here means also
   solving portalling — a new primitive on the critical path of a layout fix.
3. On a phone a full-width list beats a 200px popover, and it is the owner's own "more cmds in
   a dedicated cmd palette".

Cost, stated plainly: New note is two interactions instead of one. `c` and `Ctrl+K` still exist
on desktop.

Theme, Back up and the workspace-columns selector moved into the palette. Columns in particular
was a select occupying toolbar width for a preference changed roughly never, and meaningless
below 40rem where the workspace is forced to one column anyway.

## Settings — a mirror, not a form

New `config` dispatch arm + `SettingsPanel.svelte`
(`shots/2026-07-19-android-settings.png`). Shows the vault list file and whether it is
writable, every vault with **its path** — on the wire since `list_vaults` existed and rendered
nowhere until now — its restic repo, whether git and `RESTIC_PASSWORD` are present, and the
`FM_*` overrides in effect.

**Read-only by construction, not by preference.** `vaults::save` is append-only and never
rewrites an existing entry, so a field offering to change a vault's path or restic repo would
silently do nothing. Remotes and identity stay in the backup panel, where they are genuinely
editable. The arm shells out to nothing, unlike `backup_status`, which runs `git ls-remote` per
vault and is the slowest command in the app — opening Settings must never hit the network.

**It immediately reported something true and unwelcome:** *"No config directory on this
machine, so there is nowhere to save a vault list."* On Android `config_dir()` falls to the
catch-all arm, which reads `FM_CONFIG_DIR` — and nothing sets it. **A phone cannot persist a
vault list today.** The eventual shell has to supply that directory from the platform. Found by
building the screen that shows it.

## Two bugs found on the way

- **`Ctrl+C`/`Cmd+C` outside a text field created a note.** Only the `k` branch of the global
  key handler checked for a modifier; `c` did not, so copying a selection from a board pane
  silently captured. Fixed.
- The palette chooses on **`mousedown`**, not click (so focus never leaves its input). The UI
  tests drove it with `fireEvent.click`, which found the row and did nothing. Worth knowing
  before writing any test against it.
