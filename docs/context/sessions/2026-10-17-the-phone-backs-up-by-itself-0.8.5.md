# 2026-10-17 — the phone backs up by itself, and a lab of three devices; v0.8.5

> A snapshot, never updated. Current state is `features.md` and `known-issues.md`; what is left is
> `outstanding.md` §2.26.

## How it started

The owner was thinking aloud about treating one person on several devices differently from
collaboration — *"it was enough that the two are kept as uptodate as possible"* — then found a note
(*Books*) written on the phone days before and missing on the laptop: *"I forgot to back it up on
the phone but it stayed so for days!"*

## What was found

- **The phone had never backed up by itself.** Android serves the page from `tauri.localhost`;
  `isRemote()` read that as a paired tablet, so `autoBackup` returned on every tick. History showed
  the phone's last send on 11 Oct 10:29 and 46 changes held until a press on 17 Oct — while it got
  the laptop's changes five times. A press worked because Back up does not ask `isRemote`.
- The same check hid, on the phone only: the switch, Settings' backup half, *New vault…*, moving a
  vault, the food settings.
- **Why no test saw it:** `asPhone()` left jsdom's `localhost`, and the auto-backup wiring sat
  behind `import.meta.env.PROD`, which no test build sets — so no test had ever run it.

## What changed

- `remote.ts`: false inside the Tauri shell. `asPhone()` serves from `tauri.localhost`.
- `runsByItself()` in `autoBackup.ts`; `App.autoBackup.test.ts` runs the whole app on both
  devices and expects a send with no press. Both new tests were seen to fail with the bug put back.
- `pixi run collab-lab start|status|checklist|stop|reset|smoke` (`ci/collab-lab.sh`): Ana on two
  devices, Ben on a third, real `fm-serve`s on a bare repo under `~/.cache`. Its first run found that
  a hand-seeded repo makes a second device's first get refuse over its own `.gitignore`; the lab
  now lets the first device set the vault up through the app, as at home.
- `desktop.yml` runs `collab-lab smoke` after the Linux build of every release.

## How it was checked

UI suite (954), `check-ui`, `fmt-ui`, `checks`; the smoke locally (12 checks, nothing left
running). **Not on a phone** — the fix reaches it with this release.
