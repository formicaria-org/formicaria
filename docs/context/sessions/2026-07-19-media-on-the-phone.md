# 2026-07-19 — Media capture on Android: the byte path, and looking at the thing

**The ask:** while writing on a phone, take a photo/video/audio recording or pick from the
gallery, and have it land at the cursor. The owner's framing: this decides whether the mobile app
is usable at all.

## The finding that reshaped the plan

**Media did not work on Android in either direction, and it was never a missing camera.** All
four measured, not assumed:

| | |
|---|---|
| `ingestFile` | POSTs to `/api/ingest` — an HTTP route only `fm-serve` has. **No server on a phone.** |
| `resolve_asset` | returns raw bytes; the shell runs every reply through `String::from_utf8`, so a JPEG **fails to decode** |
| `assetUrl` | points at `/api/blob/…` — same missing route |
| `streamsBlobs` | was `import.meta.env.PROD`, so a Tauri build **claimed to stream** and then asked a server that was not there |

The shell registered **no protocol handler at all**, despite `lib.rs`'s own header describing one
("ruling 7"). So the first milestone was a *byte path*, not a camera — building capture on a
transport that cannot carry the result would be roofing an unbuilt house.

## What shipped

- **`fmblob://` URI scheme** in the Android shell, serving blobs through the same `resolve_asset`
  the desktop uses. A protocol handler rather than base64 because it **streams** — a video should
  not be held in memory twice on the way to the screen — and the webview can seek.
- **`fm_ingest`**, taking the file as a **raw IPC body**. Base64 would inflate a photo by a third
  and copy it several times.
- **Capture menu = a plain `<input type="file">`.** Android turns `capture="environment"` into
  the system camera Intent and a bare `accept` into the picker. **No native code, no plugin, no
  permission of ours.**

## The spike, answered from source rather than by trying it

wry 0.55.1 `src/android/kotlin/RustWebChromeClient.kt:272` implements `onShowFileChooser`,
including multi-select and the capture Intent. And its permission check is *in our favour*:

```kotlin
hasPermissions(activity, CAMERA) || !hasDefinedPermission(activity, CAMERA)
```

Satisfied when CAMERA is granted **or is not declared at all**. This app declares only `INTERNET`,
so capture reaches the camera app directly — **declaring CAMERA would make it worse**, adding a
prompt for something the camera app already asks about. The manifest stays as it is.

Screenshots are deliberately absent: Android's own is a hardware gesture that lands in the
gallery, and "From the library" inserts it.

## The lesson: run it and look at it

216 tests passed while three visible bugs shipped. All three were found by installing on the
emulator and taking a screenshot:

1. **The gear rendered as blank space.** Every `ICONS` value is inner SVG markup; a bare path `d`
   string with no `<path>` around it draws nothing.
2. **Both label-hiding rules never fired.** They were written for `[data-layout='single']`, and
   `auto` is the default — so a phone matched neither. **Every narrow-layout rule in `App.svelte`
   must be written twice**, once for `single` and once inside the `max-width` query for `auto`.
   Writing one half is silent.
3. **The action list laid out horizontally**, overflowing both edges, because `.actions` was
   already the dialog's footer button row and that row is `display: flex`. A CSS class collision
   inside one component.

None of these is reachable by a unit test, and all three were obvious in one screenshot.
`adb exec-out screencap -p` works headless on the emulator and is now the cheapest check
available for anything visual.

## Also this session

- The command palette was **deleted**. It had become a second Settings — it carried `Columns` and
  the theme toggle, which Settings also owned, so two half-menus could disagree about one
  preference, and it reopened stuck on whatever filter a "+" button had left. Settings is the one
  surface: actions first ("Do something", grouped and filterable), preferences after.
- **A dedicated Android launcher icon** (`mobile/icon-source.svg`). Android masks icons to a
  circle/squircle and only the inner ~66% survives; the favicon fills its canvas, so the ant lost
  its antennae. The scale was *measured*: the artwork reaches 1.12× the half-canvas, so a first
  attempt at 0.62 still landed at 69.5% and would have been clipped anyway. 0.55 puts it at 64.3%.
- **Contributors** stopped inventing collaborators: the `formicaria` placeholder was being counted
  as a person, and the list deduplicated by name rather than email so one human signed two ways
  counted twice.
- **Keyboard shortcuts**, rebindable in Settings. First defaults used `Ctrl+[`/`Ctrl+]`,
  unreachable on an Italian layout; second used `Ctrl+1/2/9/0`, which the **browser** owns (tab
  switching, zoom reset). Layout-safe is necessary and not sufficient. Now `Ctrl+.`/`Ctrl+,`, with
  a `reserved()` check that warns in Settings and a test barring them from the defaults.

## Still open

- **Capture is unverified on a real device.** The emulator has no camera and the owner needed
  their phone back. The file input, the Intent and the ingest round trip have not been exercised
  end to end.
- **Video size** against restic on mobile data, and the fact that a phone vault lives in
  app-private storage wiped on uninstall — losing the only copy of a photo is worse than losing
  notes. Restic is not available on Android, so that gap is currently unclosed.
