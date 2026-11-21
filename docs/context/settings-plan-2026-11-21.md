# One Settings for the whole app (plan, 2026-11-21)

> **A plan, not a ruling.** The decision entries in §8 are written first, each answering the four
> questions; nothing below is built before them. Source of the facts: a read-only inventory of
> `ui/src/lib/settingsMap.ts`, `SettingsPanel.svelte`, the feature pages, the Privacy sheet, the
> menus, the browser kit, the Android shell and `decisions.md` (2026-11-21).

**The owner:** *"we need a nice plan because we messed with settings many times. One settings only for
the entire app, exposing all the features in a nice, documented and easy to use interface"*; *"we do see
the firefox settings, no way to have them together with our fm ones? At the end we are packaging a single
app, regardless of the dependencies."*

## 1. Where things stand

**Ten categories** (`settingsMap.ts:8-23`): Look, Keyboard, Clock, Vaults & backup, Food & reminders,
Assistant, Privacy, Browser, Connections, About & updates. One category at a time, a search over
sections' words (2026-10-11).

**What is wrong, found in the inventory** (file:line in the inventory report; the fixes are in §6):

| # | Problem |
|---|---|
| F1 | **On the computer, `/f/settings` has controls that do nothing** (`Feature.svelte:284-296`): Layout and Columns are wired to nothing, keyboard rebinding is read by no page, there is no actions window (`commands=[]`), and choosing a theme file probably does nothing |
| F2 | **Firefox's own settings (`about:preferences`) are reachable and win over fm's**: the kit sets defaults only, so a Firefox switch can contradict a formicaria privacy switch, silently |
| F3 | **The rule "Settings holds no action"** (2026-10-15) is broken in a dozen places (move/remove vault, pairing, Check now/Update/Go back, Read now, Forget web data, Delete the diary, Sync now, New password); only unused attachments was amended in |
| F4 | **Several doors to one thing**: the Privacy sheet has four (≡, `/f/privacy`, the ant, Settings → Privacy), and only the one opened beside a page shows that site; updates are offered in four places |
| F5 | **The actions window ("Do something") opens only from a keyboard shortcut**: none on the phone, yet the manual tells phone users it is in Settings. `keys.LABELS.palette` says "Settings" while it opens "Do something" |
| F6 | **Names drift**: the manual says *Appearance* (the category is *Look*), *Study assistant* (it is *Assistant*), Welcome says *Back up* for what is Settings → Vaults & backup |
| F7 | **Mixed categories**: *Food & reminders* holds trackers, watch files, reminders and "show Music and Videos"; *About* holds the vault list; *Clock* is a category of one field |
| F8 | **Things with no place in Settings**: "Create in" (default vault) and "which vaults to show" (toolbar only, absent on the phone); browser shortcuts (Alt+Shift+X/K/A/S) neither listed nor rebindable |
| F9 | **Ad blocking in two places with two names** (Privacy *Block ads* = uBlock Origin Lite; Browser *Install uBlock Origin*) |
| F10 | **No row says where it applies**: this device (localStorage, `privacy.json`, `clock.json`, `browser.json`), this vault (`vault.json`), or everywhere |
| F11 | **Manual**: no Settings page in `SUMMARY.md`; no link beside Privacy, Usage diary, Time zone, The bar, Subsonic, Vault list |
| F12 | **Phone**: Keyboard shown where it is near useless; the browser's web settings exist only as Privacy; no native settings screen (all through the page) |

**History, why this keeps happening:** 2026-10-11 made one page, then one category at a time and a
separate actions window opened by shortcut; 2026-10-15 split *Vaults & backup* out and ruled it free of
actions; 2026-11-17/18 moved Settings to the window's top right and the ant; each later feature added a
section where it fit that day. No entry owns the whole map, so each addition was justified locally and
the sum drifted. **This plan is that map, and §7 makes it hold.**

## 2. Principles

1. **One Settings, every device, the whole app** — formicaria's own settings *and* the browser's, in
   formicaria's words. Firefox is a part, not a second app: its settings page is never shown (§5).
2. **The same categories on every device**, in the same order and names. Something a device cannot do
   is said in its place ("On the computer"), not hidden — except where it would only clutter a phone
   (Keyboard, §4).
3. **Every row says where it applies**: *this device*, *this vault*, or *all your devices* (a small
   label), because that is what a person needs to know before changing it.
4. **An action sits beside the setting it acts on** (Check now beside *Look for newer versions*,
   Remove beside a vault). A destructive one asks first and says what can be undone. **Supersedes**
   "Settings holds no action" (§8).
5. **One door per thing.** A thing has one home in Settings; anywhere else it appears (the ≡ menu, the
   ant, a banner) is a link *to that home*, or the in-context form of it (the Privacy rows for the site
   in front), never a second copy that can disagree.
6. **Documented where it is**: every section has a *?* link to its manual section; the manual has one
   Settings chapter whose headings are the categories, checked against `settingsMap.ts` by CI (§7).
7. **Nothing does nothing.** A control shown on a page works on that page; a test walks every control
   on every kind of page (§7).

## 3. The categories

Eleven, by what a person comes to do. *Italic* = new or moved.

| Category | What it holds | From |
|---|---|---|
| **Look** | Theme, accent, text size, font, theme files; layout (one view/tiled), columns; *the bar's place (phone)* | Look |
| **Notes & vaults** | Vaults (create, move, remove, add back), *the vault new notes go into*, *which vaults to show*, *your name and email* (once, used by every vault), *the vault list's place* | Vaults & backup, toolbar, About |
| **Backup & sync** | *Backs up by itself*, where notes are copied to (remote, token), snapshots (restic repo, password), *which attachments travel* (size, music, videos), *Back up now / Get changes / Take a snapshot* | Vaults & backup, Back up menu |
| **Storage** | *Space used per vault*, unused attachments (look, remove, put back) | Vaults & backup |
| **Trackers** | Show Food, Fitness, Health, Music and Videos on this device; *targets' defaults*; watch files and Gadgetbridge | Food & reminders |
| **Time & reminders** | Time zone, reminders on this device, *calendars* | Clock, Food & reminders, Connections |
| **Media** | *Save media's defaults* (keep as, quality, into), your music in other players (Subsonic), *the media tools' state* | Connections, About |
| **Browser** | Everything in §5 (passwords, cookies and site data, site permissions, downloads, zoom and fonts, languages, notifications, tabs on the side, after a selection, open formicaria in it) | Browser, Firefox |
| **Privacy** | Protections (trackers, ads — *one switch*, cookie pop-ups, tracking links, do-not-sell, secure only, more sites), search engine, encrypted lookups, sites with protections off, Forget web data, usage diary; *phone: block trackers in other apps* | Privacy |
| **Assistant** | On/off, this device's name, model, transcription, better listening | Assistant |
| **Connections** | Share with another device (pairing, which vaults, disconnect), mail for meetings | Connections |
| **Keyboard** | *Every* shortcut, the browser's included (Alt+Shift+X/K/A/S), rebindable; *computer only* | Keyboard |
| **About & updates** | Version, look for newer versions, Check now / Update / Go back, this machine's facts, environment overrides | About & updates |

(Twelve with Keyboard; it is not listed on a phone or a tablet without a keyboard.)

**Above the categories**, always: the search (it also finds *actions* by name), and **Do something…**
— the actions window, now a visible button on every device (F5), with its shortcut beside it.

## 4. Each row, the same shape

`Label` · control · *scope* (this device / this vault / all your devices) · one line of what it does ·
*?* to the manual · when relevant, the action beside it and what it can undo. A row that a device cannot
use says where it can ("On the computer — formicaria's browser"). Read-only facts sit last in their
category, under *Details*.

## 5. The browser's settings, inside formicaria's

**What a person can set** (the rest stays at formicaria's defaults and is not shown — telemetry,
Pocket, sponsored content, studies, AI chat, as today):

| Row | Firefox underneath |
|---|---|
| Saved passwords: Open the list; Import from a file…; Sync with a Firefox account… | `about:logins`, `LoginCSVImport`, `about:preferences#sync` (opened by the kit) |
| Cookies and site data: how much is used; Clear for one site; Clear all | `SiteDataManager` |
| Site permissions: camera, microphone, location, notifications — per site, change or forget | `SitePermissions` |
| Downloads: the folder; always ask | `browser.download.*` |
| Zoom, fonts, minimum text size | `layout.css.devPixelsPerPx`-free prefs: `browser.zoom.*`, `font.*` |
| Languages for pages | `intl.accept_languages` |
| Notifications while a tab is closed | `dom.push.enabled` (already) |
| Tabs down the side; after a selection; open formicaria in it | `browser.json` (already) |
| Search engine; DNS over HTTPS; proxy (under *More*) | the Privacy sheet's engine (already); `network.trr.*`; `network.proxy.*` |

**How the page reaches them** — a narrow bridge, not the server: the kit (autoconfig, browser privilege)
registers a window actor for **formicaria's own `/f/settings` page only**, exposing a short allowlist
(`get`/`set` of the prefs above, open the password list, import a CSV through a file picker, list and
clear site data and permissions). Nothing else is reachable from a page; a test pins the allowlist.

**Firefox's settings page is never shown**: the kit sends any opening of `about:preferences` (menu,
shortcut, link) to formicaria's Settings → Browser, and **formicaria's switches become the truth**:
the kit sets them as user values on change, not defaults, so `about:preferences` cannot contradict them
(F2). `about:logins` stays reachable from *Open the list* (it is the list itself).

**On the phone** the same Browser category shows what Android's web view allows (clear site data,
notifications per site) and says the rest is the computer's.

## 6. The fixes, mapped

| Fix | For |
|---|---|
| `/f/settings` gets the real handlers (layout, columns, keymap, theme file) or does not show the control; the feature pages read the keymap | F1 |
| Browser category + bridge + `about:preferences` redirected + user-value prefs | F2, §5 |
| Actions beside their settings; destructive ones confirm (superseding 2026-10-15) | F3 |
| One home each: Privacy's home is Settings → Privacy; ≡, the ant and `/f/privacy` open it (with the site in front when there is one). Updates' home is About & updates; the banner and the kit bar link there and keep their one press | F4 |
| *Do something…* button in Settings' header on every device; `keys.LABELS.palette` = "Do something" | F5 |
| Names: one table (`settingsMap.ts`), the manual and Welcome use it; a CI check refuses a "Settings → X" the table does not have | F6 |
| The categories of §3 | F7, F8 |
| One *Block ads* switch (Privacy); the Browser row removed | F9 |
| Scope labels | F10 |
| A *Settings* chapter in the manual, one section per category; a *?* on every section (CI: a section without one fails) | F11 |
| Keyboard computer-only; Browser on the phone with what the web view allows | F12 |

## 7. How it stays right

- **`settingsMap.ts` is the map**: category, section, scope, manual anchor and platforms per section. A
  section not in the map does not render (today it silently vanishes, a known trap) — and CI fails.
- **CI checks**: every section in the map has a manual anchor that exists; every "Settings → X" in
  `docs/src/user/` names a real category; no control on `/f/settings` is wired to a no-op.
- **A usability persona, *someone going through Settings***, in formicaria's browser: opens each
  category, checks each control has a label, a scope and a working *?*; changes one setting per category
  and sees it take effect (theme, a privacy switch reaching the browser, a browser pref read back
  through the bridge, a vault's attachment rule); opens `about:preferences` and lands in Settings →
  Browser; searches for an action and runs it. The same walk on the phone in `phone-media-check`.
- **The decision index** gets a `#settings` subject; any entry adding a setting must name its category
  and scope.

## 8. Decision entries to write first

1. ***One Settings for the whole app, the browser's included*** `#ui` — principles 1–3, the categories of
   §3; supersedes the category list of 2026-10-11 and *Vaults & backup is a category of its own*
   (2026-10-15 later), whose amendment folds in.
2. ***An action sits beside the setting it acts on*** `#ui` — supersedes *Settings holds no action* and
   *nothing in it is an action* (2026-10-15), and *the actions window opens only by shortcut*
   (2026-10-11): *Do something…* is a button.
3. ***Firefox's settings are formicaria's*** `#ui` `#toolchain` — the bridge (window actor, allowlist,
   `/f/settings` only), `about:preferences` redirected, formicaria's switches as user values; amends
   *formicaria's browser is a kit applied to Mozilla's ESR* (2026-11-15) on defaults vs user values.
4. ***One door per thing*** `#ui` — Privacy's and updates' homes; the other places link.
5. ***The map is checked*** `#toolchain` — the CI checks of §7.

## 9. Order of work

| Phase | What | Shows |
|---|---|---|
| S0 | The entries of §8 | — |
| S1 | `settingsMap.ts` as the map (scope, anchor, platforms); the categories of §3; names; the manual's Settings chapter; CI checks | Same controls, new places, every *?* works |
| S2 | F1 (`/f/settings` controls that do nothing), F5 (*Do something…*), F9 (one ad switch), scope labels | Nothing dead; actions reachable on a phone |
| S3 | The browser bridge and the Browser category (§5); `about:preferences` redirected; passwords import | Firefox's settings gone, its useful ones in formicaria's |
| S4 | One door per thing (Privacy, updates); the desktop toolbar's vault preferences moved | — |
| S5 | The *going through Settings* persona (computer and phone) in the release gate | The map held by a test |

Each phase is a release, through the full gate.

## 10. Questions for the owner

1. **The categories of §3** — eleven (twelve with Keyboard): merge or split any?
2. **Firefox account sync** in Settings → Browser — show it (passwords through Mozilla, end-to-end
   encrypted), or leave it out?
3. **Keyboard on a phone** — hide it (proposed), or keep it for tablets with a keyboard?
