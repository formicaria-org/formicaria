# Handwriting devices for formicaria: survey, 2026-11-01

Which writing device best suits a vault whose notes belong only to their user? This records what
was read, what was run, and what is still unknown. Vendor pages and reviews were read on the date
above; prices and firmware behaviour change.

## Recommendation

1. **No device yet:** a paper notebook, a photo and `/transcribe`. It needs no new code and has the
   smallest privacy surface, though the phone's cloud photo backup must be off.
2. **If a device is wanted: Supernote**, never signed in, Wi-Fi off, firmware and files over USB.
   Nomad (7.8") by default; Manta (10.7") for full-page writing or reading PDFs.
3. **reMarkable Paper Pure** if writing feel matters more than a smaller privacy surface.

No closed-firmware device can *guarantee* that notes stay private. Supernote is ahead because of
its documented behaviour and offline paths, not because anyone has audited current firmware.

## Measured against formicaria

Run with `target/release/fm` (built 2026-10-24, before the checkout at `v0.26.0-1-g24fcfa3`), in a
throwaway vault, on **hand-made files**. These were not real device exports.

| Input | Result | Searchable characters |
|---|---|---|
| PNG page, 1200×1600 | Added as `image/png` | 0 |
| Stroke SVG, 251 kB, 3,000 paths | Added as `image/svg+xml` | 0 |
| PDF with a text layer | Added; found by `fm search` | 31 |
| PDF with strokes only | Added | 0 |
| Fake `.note` (text header and zero bytes) | Added as `text/plain` | 2,072 (not meaningful) |

`fm verify` reported 0 errors and 0 warnings. Each add took 10–50 ms.

What this shows:

- **Exports are storable now.** Strokes-only PDFs and pictures are stored, but cannot be searched
  until something adds text.
- **Pictures and strokes-only PDFs have blank titles in `fm list`.** An importer should set them.
- **Adding the same file twice reuses the blob but creates a second note.** A repeat device import
  therefore has to track what it has already brought in; `fm add` alone is not enough.
- **A real binary `.note` is not usable as an attachment.** Keep it as the original and convert it.

SVG is deliberately not served for direct inline viewing, because it can carry script
(`crates/fm-app/src/wire.rs`, `crates/fm-serve/src/blob.rs`). PNG is the safer page format, and it
is also what `/transcribe` is documented to read.

## Not run

- **Converters on real files.** Installing and running `supernotelib` and `rmc` was blocked by the
  session's permission system. Their conversion claims below come from their READMEs and source.
- **`/transcribe` on handwriting.** `phone-image-to-text-alternatives-2026-10-10.md` still lists the
  user's own handwritten pages as the missing test.
- **Network capture of either tablet.** No device was available.
- **Latency.** No independent side-by-side measurement was found. The Paper Pure figure below comes
  from a third-party site, and no comparable Supernote figure was confirmed.

## Hardware

| | Supernote Nomad | Supernote Manta | reMarkable Paper Pure |
|---|---|---|---|
| Screen | 7.8", 1404×1872, 300 PPI | 10.7", 1920×2560 | 10.3", 1404×1872, Carta 1300 |
| Processor | RK3566, quad-core 1.8 GHz | Rockchip quad-core | Dual-core Cortex-A55, 1.7 GHz |
| RAM / storage | 4 GB / 32 GB | 4 GB / 32 GB | 2 GB / 32 GB |
| microSD | Yes | Yes, up to 2 TB | No |
| Battery | 2,700 mAh, user-replaceable | 3,600 mAh, replaceable | Up to 3 weeks claimed; user-replaceable per one third-party source |
| Weight | 266 g | 375 g | 360 g |
| Pen | Sold separately, from about $65 | Sold separately | Marker and 6 tips included |
| Price | From $359 | $505 | $399 |

Sources: [Supernote Nomad page](https://supernote.com/products/supernote-nomad),
[CGMagazine on the Nomad](https://www.cgmagonline.com/review/hardware/supernote-nomad-tablet-review),
[New Atlas on the Manta](https://newatlas.com/mobile-technology/supernote-manta-eink-enote-tablet),
[Notebookcheck on the Paper Pure](https://www.notebookcheck.net/reMarkable-Paper-Pure-launches-as-the-successor-to-the-reMarkable-2.1290609.0.html),
[Expert Reviews on the Paper Pure](https://www.expertreviews.co.uk/technology/tablets-ereaders/remarkable-paper-pure-review).
Weights and the Manta's screen and battery come from third-party listings and search excerpts.

## Software and requirements

| | Supernote | reMarkable |
|---|---|---|
| System | Closed, Android-based (Chauvet) | Closed, Linux-based |
| Account needed | No, per an owner running offline | A guide describes use without one once unpaired; first setup not confirmed |
| Default design | Offline-capable; cloud optional | [Built to keep everything synced to its cloud](https://remarkable.jms1.info/faq/cloud.html) |
| Handwriting to text | Offline after [one online activation](https://support.supernote.com/handwriting-recognition) | [Needs Wi-Fi and an account](https://support.remarkable.com/articles/Knowledge/Convert-handwritten-notes-into-text) |
| Firmware without Wi-Fi | [Official manual USB update](https://support.supernote.com/en_US/change-log/how-to-update-your-supernote) | Not confirmed |
| Getting files out | [Appears as storage over USB](https://foo.zone/gemfeed/2026-01-01-using-supernote-nomad-offline.html) | [USB web interface at 10.11.99.1](https://lizard-spock.co.uk/remarkable-local-file-transfer-without-the-cloud.html), or SSH |
| Shell access | Community rooting guides | [Official developer mode](https://developer.remarkable.com/documentation/developer-mode); factory-resets, weakens device security, shows a warning on every boot. Page describes the Paper Pro; Paper Pure not confirmed |

## Ownership evidence

**Supernote**

- [Its policy](https://supernote.com/policies/privacy-policy) says device logs are kept locally and
  sent only with consent: by actively submitting logs, or with "Usage and Diagnostics" on. Those
  logs can include file names, pen events and stroke counts. Firebase is used under that setting.
- A [security write-up from 2022](https://github.com/TA1312/supernote-a5x) found nothing obviously
  compromising on the older A5 X, and noted partly obfuscated Tencent components in some apps. It
  is one person's review of an old model and old firmware.
- Ratta [reportedly promised open-source Linux firmware](https://foo.zone/gemfeed/2026-01-01-using-supernote-nomad-offline.html),
  but one owner says it had not appeared after more than a year. Do not buy on that promise.

**reMarkable**

- A [community FAQ](https://remarkable.jms1.info/faq/cloud.html) says software 3.12 and later
  connects to Memfault to send telemetry and analytics. Whether an unpaired or offline tablet does
  so was not confirmed, and the FAQ may be dated.
- The company's [privacy policy page](https://support.remarkable.com/s/article/reMarkable-s-privacy-policy)
  loads, but its text could not be read by script. **It has not been reviewed here.**
- Its support site says you can [stay offline and store documents only on the device](https://support.remarkable.com/articles/Knowledge/Pair-your-reMarkable-with-the-cloud).

**Not reviewed for either vendor:** terms of service, and encryption of notes on the device. A lost
tablet may be readable by whoever finds it.

## Fit with formicaria

Formicaria has no device importer. Its import code handles Logseq and Obsidian only
(`crates/fm-core/src/import.rs`). A device importer would suit three existing rules: the source
folder is read-only, heavy tools are shelled out to rather than linked (`CONTRIBUTING.md`), and
attachments are stored by content hash.

| Converter | License | Input | Output, per its docs | Risk |
|---|---|---|---|---|
| [supernote-tool](https://github.com/jya-dev/supernote-tool) | Apache-2.0 | `.note` | PNG, SVG, vector PDF, TXT from recognized notes | Unofficial; rejects unknown signatures in its default strict mode, so a firmware update can break it |
| [rmc](https://github.com/ricklupton/rmc) with [rmscene](https://github.com/ricklupton/rmscene) | MIT | `.rm` v6 | SVG, PDF, simple Markdown; points carry pressure, width, speed | Unofficial |

Suggested shape, computer only since both tools are Python: read the device folder, convert each
page to PNG, attach it with the original, track what has been imported, then leave text to
`/transcribe`. Nothing lands in a note without review, as today.

## Connectivity and automatic saving

| | Supernote | reMarkable Paper Pure |
|---|---|---|
| Wi-Fi | Yes; [dual-band on the Manta](https://newatlas.com/mobile-technology/supernote-manta-eink-enote-tablet) | [Dual-band](https://www.liseuses.net/remarkable-paper-pure/) |
| Bluetooth | 5.0 on the Manta; firmware notes describe keyboards and page turners. No file-transfer use found. Nomad version not confirmed | Sources conflict: [5.1 in one review](https://www.liseuses.net/remarkable-paper-pure/), [absent on a comparison site](https://comparisontabl.es/e-readers/remarkable-paper-pure/) |
| USB | Storage access, per an owner on Linux | USB network adapter and web interface |
| Local Wi-Fi transfer | [Browse & Access](https://ratta.helpjuice.com/wi-fi-transfer): opened on the device, same network required | None confirmed besides a self-hosted cloud |
| Self-hosted sync | Device has a Private Cloud setting; [one server is unofficial](https://pypi.org/project/supernote/0.22.0/) and untested here | [rmfakecloud](https://github.com/ddvk/rmfakecloud) lists the Paper Pure; tested up to software 3.27.1 |

Any Wi-Fi route means the tablet joins a network, so to keep a closed device off the internet it
needs a firewalled network with no route out. USB avoids that entirely.

**Automatic saving is feasible, but nothing has been built or run.** A tool separate from the
released app is the smaller first step: on USB connection, copy the device's files read-only,
convert each page to PNG, attach it with `fm add`, and keep a ledger of what it has added, because
`fm add` makes a second note on a repeat. Follow the import rules: add-only, never rewriting a note
the user may have edited (`decisions.md`, *An import converts; adoption renders*).

### Wi-Fi to a computer running formicaria

The goal here is no cable: tablet, Wi-Fi, a folder on the computer, formicaria, then git. None of
these routes has been run; figures are from GitHub on the survey date.

| Route | Runs on the tablet | Runs on the computer | Maturity | Catch |
|---|---|---|---|---|
| [Syncthing](https://docs.syncthing.net/users/syncing.html) with an [Android build](https://github.com/researchxxl/syncthing-android) | Sideloaded app ([official toggle](https://code.adamdimech.au/sideload-apps-onto-a-supernote-device/)) | Syncthing | 89,984 stars; owners report it working on the Nomad | Background behaviour and battery cost on e-ink unconfirmed |
| [supernote-sync](https://github.com/jbchouinard/supernote-sync) | Nothing; uses [Browse & Access](https://support.supernote.com/Tools-Features/wi-fi-transfer) | Python tool polling port 8089 | 4 stars, 1 contributor, last push 2025 | Wi-Fi transfer must be on; access control on that port unchecked |
| [Official Private Cloud](https://support.supernote.com/setting-up-your-own-supernote-private-cloud-beta) | Built in; Nomad and Manta need Chauvet 3.24.39 beta or later | Docker on Linux, plus a TLS reverse proxy | Vendor-supported, beta | Heaviest; LAN-only use without internet unconfirmed |
| [allenporter/supernote](https://github.com/allenporter/supernote) | Built-in Private Cloud setting | Python or Docker, SQLite | 82 stars, active | Unofficial protocol copy; has optional cloud AI features to leave off |
| [rmfakecloud](https://ddvk.github.io/rmfakecloud/remarkable/setup/) (reMarkable) | Proxy or `/etc/hosts` change on the device | Go server | 1,369 stars | Needs device modification, so developer mode on current models |

Syncthing fits best: one direction ([send-only on the tablet, receive-only on the computer](https://docs.syncthing.net/users/foldertypes.html)),
with global discovery and [relays](https://docs.syncthing.net/users/relaying.html) off. Any of these
leaves a closed device on a network, so that network should have no route to the internet.

**Measured: `fm add` from the command line left no `.git` in a new vault**, so it does not commit by
itself. Whether a running `fm-serve` later commits such a note was not tested. By default `blobs/`
is also outside git, so page images only reach a remote if the vault's attachment limit is set.

Points to settle first:

- Whether `fm add` is safe while `fm-serve` has the same vault open. Not checked; a quick search of
  `fm-core` found no cross-process lock.
- A built-in version would extend the deferred "watched inbox" (`known-issues.md`). It should poll
  rather than use a file watcher, for the reasons recorded there, and needs a `decisions.md` entry
  if it widens what ships.
- `/transcribe` is a per-note command and its output is reviewed, so text will not appear unattended.
- Both converters are Python, so this route is computer-only; the phone keeps Share.

## Options set aside

- **Moleskine, Neo, Livescribe:** need dotted paper and vendor apps.
- **Nuwa Pen:** no independent accuracy test found; its app includes cloud storage and an AI
  assistant, so its "on-device" claim needs checking.
- **PineNote:** the most open hardware, but availability could not be confirmed and it is not a
  finished notebook.
- **Kindle Scribe, Boox:** no firm evidence of an offline, account-free route; Boox's device privacy
  policy could not be confirmed.
- **Rnote, Xournal++, Saber:** the most open software, but they need separate stylus hardware.

## Checks before trusting any device

1. Connect it once through a router you can log, with no account, and record what it contacts.
2. Confirm "Usage and Diagnostics" is off and stays off after a firmware update.
3. Export ten real pages, convert them on the computer, and run `/transcribe` on them.
4. Keep the return window open until those three are done.
