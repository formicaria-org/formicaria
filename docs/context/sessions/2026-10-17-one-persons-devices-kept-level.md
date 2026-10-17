# 2026-10-17 (afternoon) — one person's devices kept level

> A snapshot, never updated. Current state is `known-issues.md` (*Backing up by itself*); the ruling
> is `decisions.md`, *one person's devices are kept level*; what is left is `outstanding.md` §2.26.

## How it started

After 0.8.5 the owner asked how the app now deals with one person on several devices, and chose
the four small items: *"do 1-4"*.

## What changed for a person

1. **A chip says when changes are not leaving this device** — the last automatic try failed (the
   reason in the hover), or changes have waited over ten minutes with the switch on. The second
   needs nothing to have run, which is what the five silent days on the phone needed.
2. **Backing up at more moments**: coming back after half a minute away (past the gap and the
   phone's five-minute floor), starting to edit a note another device changed, leaving a note
   written in (a 20 s gap), the network coming back (past the failure wait). Each still calls
   `syncVault` — one path, none that only gets.
3. **An open note takes what arrived** when it holds nothing unsaved — the conflict the owner kept
   meeting: a note opened before a get, typed into after it, refused as stale.
4. **No write when nothing changed** — `update_body`, `set_property`, `set_properties`. A stale pane
   holding exactly the disk's text is no longer refused either.

## How it was checked

Every new test was seen to fail with its piece removed: `unchanged_write.rs` (4 of 5; the fifth
pins that real edits still write), `NotePanel.keepLevel` (taking what arrived), `App.autoBackup`
(the chip, both signals; coming back). UI 974, Rust 1127 + native-git 73, clippy, fmt, checks.
**Not on a phone and not between two real devices.**
