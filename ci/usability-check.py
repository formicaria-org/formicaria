#!/usr/bin/env python3
"""**Usability checks: formicaria's browser, used as different people use it** (2026-11-17; the owner:
*"run proper usability checks simulating different sorts of users"*, after Settings and updates had
become unreachable in a browser set up by an older version).

`ci/browser-check.py` measures the kit against plain Firefox; this walks it as people do, each in a
profile of its own (never the owner's), against a real server with its own settings and vault:

- **A returning user whose ant is tucked in Firefox's Extensions menu** — the state an older version
  left one person's browser in, with Settings, Privacy and updates then out of reach on formicaria's
  pages (their address row is hidden). The ant must come back to the tab row by itself.
- **A new person on their first day** — Notes, the new tab's choices, the ant's panel, Settings with
  updates, Privacy, Save media.
- **Someone who saved a page and reads it later** — its copy must open as a page, not a download.
- **A keyboard user** — the new tab's field takes words and goes, without the mouse.
- **Someone who counts on the protections** — the privacy test site (`ci/privacy_site.py`): trackers
  refused, GPC sent, tracking parts stripped, strict's fingerprinting protection, encrypted lookups.
- **Someone who just updated** — the browser still holds the previous release's kit; at the first
  start the new kit must be in place, Settings in the tab row, nothing waiting for another restart.

Prints each check and its result; exits 1 if any failed. Run: `pixi run usability-check`.
"""
import base64
import importlib.util
import json
import os
import shutil
import subprocess
import sys
import time
import traceback
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("browser_check", ROOT / "ci" / "browser-check.py")
bc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bc)

ANT = "browser_formicaria-org_github_io-browser-action"
ADDON = "browser@formicaria-org.github.io"
SERVER = bc.SERVER
results: list[tuple[str, str, bool, str]] = []


def check(who: str, what: str, ok: bool, why: str = "") -> None:
    results.append((who, what, bool(ok), why))
    print(f"usability: {'ok  ' if ok else 'FAIL'} [{who}] {what}{'' if ok else ' — ' + why}", flush=True)


def launch(ff: Path, profile: Path, url: str, prefs: str = "") -> "bc.Marionette":
    if profile.exists():
        shutil.rmtree(profile)
    profile.mkdir(parents=True)
    (profile / "user.js").write_text('user_pref("dom.disable_open_during_load", false);\n' + prefs)
    subprocess.Popen(
        [str(ff / "firefox"), "--headless", "--marionette", "-remote-allow-system-access", "--no-remote",
         "--name", "formicaria-check", "--profile", str(profile), url],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )
    m = bc.Marionette()
    m.cmd("WebDriver:NewSession", {"capabilities": {}})
    # **Stand in formicaria's own tab, once the start has settled.** Privacy extensions open a tab
    # when installed and the kit closes it; a session standing in it lost its window ("browsing
    # context has been discarded") — twice on 2026-11-17, in two personas.
    for _ in range(30):
        time.sleep(0.5)
        try:
            hs = m.cmd("WebDriver:GetWindowHandles")
            hs = hs["value"] if isinstance(hs, dict) else hs
            for h in hs:
                m.cmd("WebDriver:SwitchToWindow", {"handle": h})
                if m.cmd("WebDriver:GetCurrentURL")["value"].startswith(SERVER):
                    return m
        except RuntimeError:
            continue
    return m


def chrome(m, script: str):
    m.cmd("Marionette:SetContext", {"value": "chrome"})
    try:
        return m.cmd("WebDriver:ExecuteScript", {"args": [], "script": script})["value"]
    finally:
        m.cmd("Marionette:SetContext", {"value": "content"})


def click_in_window(m, selector: str) -> str:
    """Click an element of the browser's own window (its toolbars) the way a mouse does."""
    m.cmd("Marionette:SetContext", {"value": "chrome"})
    try:
        el = m.cmd("WebDriver:FindElement", {"using": "css selector", "value": selector})["value"]
        m.cmd("WebDriver:ElementClick", {"id": next(iter(el.values()))})
        return "pressed"
    except RuntimeError as e:
        return f"could not click: {e}"[:300]
    finally:
        m.cmd("Marionette:SetContext", {"value": "content"})


def content(m, script: str, wait_ms: int = 0):
    if wait_ms:
        time.sleep(wait_ms / 1000)
    return m.cmd("WebDriver:ExecuteScript", {"args": [], "script": script})["value"]


def goto(m, url: str, settle: float = 4):
    m.cmd("WebDriver:Navigate", {"url": url})
    time.sleep(settle)


def wait_for(m, script: str, secs: int = 15):
    """Run `script` in the page until it answers something truthy, or `secs` pass."""
    v = None
    for _ in range(secs * 2):
        try:
            v = content(m, script)
        except Exception:  # noqa: BLE001 - the page may be navigating
            v = None
        if v:
            return v
        time.sleep(0.5)
    return v


ANT_STATE = """
  const CUI = window.CustomizableUI;
  const at = CUI.getPlacementOfWidget('%s');
  const el = document.getElementById('%s');
  const r = el ? el.getBoundingClientRect() : null;
  return { area: at ? at.area : null, visible: !!r && r.width > 0 && r.height > 0,
           navHidden: getComputedStyle(document.getElementById('nav-bar')).display === 'none',
           marked: document.documentElement.hasAttribute('formicaria-page'),
           at: window.gBrowser.selectedBrowser.currentURI.spec };
""" % (ANT, ANT)


def returning_user(ff: Path, work: Path) -> None:
    who = "returning user, ant in the Extensions menu"
    # The placements an older version left: the ant inside the Extensions menu.
    state = {
        "placements": {
            "unified-extensions-area": [ANT],
            "nav-bar": ["back-button", "forward-button", "stop-reload-button", "urlbar-container",
                        "downloads-button", "unified-extensions-button"],
            "TabsToolbar": ["tabbrowser-tabs", "new-tab-button", "alltabs-button"],
        },
        "seen": [ANT], "dirtyAreaCache": ["nav-bar", "TabsToolbar", "unified-extensions-area"],
        "currentVersion": 23, "newElementCount": 2,
    }
    prefs = (
        f'user_pref("browser.uiCustomization.state", {json.dumps(json.dumps(state))});\n'
        'user_pref("formicaria.toolbar.trimmed", true);\nuser_pref("formicaria.toolbar.trimmed2", true);\n'
    )
    m = launch(ff, work / "profile-returning", f"{SERVER}/f/group/Library", prefs)
    try:
        s = {}
        for _ in range(40):
            s = chrome(m, ANT_STATE)
            if s.get("area") == "TabsToolbar" and s.get("visible") and s.get("navHidden"):
                break
            time.sleep(0.5)
        check(who, "the ant is back in the tab row", s.get("area") == "TabsToolbar", f"it is in {s.get('area')}")
        check(who, "the ant can be seen on a formicaria page", s.get("visible") is True and s.get("navHidden") is True,
              f"{s}")
    finally:
        m.cmd("WebDriver:DeleteSession")
        bc.stop(ff)


def new_person(ff: Path, work: Path) -> None:
    who = "new person, first day"
    m = launch(ff, work / "profile-new", f"{SERVER}/f/group/Library")
    try:
        time.sleep(6)
        s = chrome(m, ANT_STATE)
        check(who, "the ant is first in the tab row, in view", s.get("area") == "TabsToolbar" and s.get("visible"), f"{s}")
        title = wait_for(m, "return document.title.includes('Notes') && document.title")
        check(who, "Notes opens", bool(title), f"title {content(m, 'return document.title')}")

        # The new tab: what + offers.
        goto(m, f"{SERVER}/f/new")
        names = wait_for(m, "const b=[...document.querySelectorAll('[aria-label=features] button')].map(x=>x.textContent.trim()); return b.length?b:null") or []
        for want in ["New note", "Notes", "Tasks", "Agenda", "Food journal", "Music", "Videos", "Discussions", "Collaboration"]:
            check(who, f"+ offers {want}", any(want in n for n in names), f"offered: {names}")
        field = content(m, "return document.activeElement && document.activeElement.getAttribute('aria-label')")
        check("keyboard user", "the new tab's field has the focus", field == "a web address or a search", f"focus on {field}")

        # The ant's panel, as the extension shows it.
        host = chrome(m, f"return WebExtensionPolicy.getByID('{ADDON}')?.mozExtensionHostname")
        if host:
            goto(m, f"moz-extension://{host}/launcher.html", 3)
            rows = wait_for(m, "const t=document.body.innerText; return t.length>3?t:null") or ""
            for want in ["Settings", "Privacy", "Save media on this page", "Help"]:
                check(who, f"the ant's panel offers {want}", want in rows, f"panel: {rows[:200]!r}")
        else:
            check(who, "formicaria's add-on is installed", False, "no add-on")

        # **Settings from the window's top right, on every tab** (owner, 2026-11-18): from a note's tab, a
        # view's and a web page's, the button at the right end of the tab row is pressed, as a person
        # would, and Settings must be the tab shown.
        for where in [f"{SERVER}/f/group/Library", f"{SERVER}/f/agenda", "https://en.wikipedia.org/wiki/Ant"]:
            goto(m, where, 5)
            seen = chrome(m, """
              const gb = window.gBrowser;
              gb.selectedTab = gb.tabs.find(t => t.linkedBrowser.currentURI.spec.startsWith(%s)) ?? gb.selectedTab;
              const b = document.getElementById('formicaria-settings');
              if (!b) return 'no button';
              const r = b.getBoundingClientRect();
              if (!(r.width > 0 && r.height > 0)) return 'not drawn';
              return 'drawn';""" % json.dumps(where))
            if seen == "drawn":
                # **Clicked as a mouse clicks** (Marionette's element click, in the window): a toolbar
                # button acts on the *command* a real click makes, which a script's `click()` does not.
                seen = click_in_window(m, "#formicaria-settings")
            time.sleep(3)
            shown = chrome(m, "return window.gBrowser.selectedBrowser.currentURI.spec")
            why = ""
            if not shown.startswith(f"{SERVER}/f/settings"):
                # What the button's handler saw: formicaria's address (from the home page) and what the
                # startup script reported.
                why = chrome(m, """
                  const home = Services.prefs.getStringPref('browser.startup.homepage', '');
                  const errs = Services.console.getMessageArray()
                    .map(x => x.message || String(x)).filter(x => /formicaria|Settings|SETTINGS|openSettings/.test(x)).slice(-4);
                  return 'home=' + home + ' | ' + errs.join(' || ');""")
            check(who, f"Settings at the top right opens from {where.split('/')[-1]}",
                  seen == "pressed" and shown.startswith(f"{SERVER}/f/settings"), f"{seen}; shown {shown}; {why}"[:900])
        n = chrome(m, f"return window.gBrowser.tabs.filter(t => t.linkedBrowser.currentURI.spec.startsWith('{SERVER}/f/settings')).length")
        check(who, "Settings pressed three times is one tab", n == 1, f"{n} Settings tabs")

        # Settings: its categories, and updates.
        goto(m, f"{SERVER}/f/settings", 6)
        cats = wait_for(m, "const t=document.body.innerText; return t.includes('About')?t:null") or ""
        for want in ["Privacy", "About & updates"]:
            check(who, f"Settings has {want}", want in cats, f"settings text starts {cats[:160]!r}")
        about = content(m, """const b=[...document.querySelectorAll('button')].find(x=>/About & updates/.test(x.textContent)); if(b) b.click(); return !!b;""")
        time.sleep(2)
        text = content(m, "return document.body.innerText")
        check(who, "About & updates says the version", about and "version" in text.lower(), f"{text[:200]!r}")

        # Privacy.
        goto(m, f"{SERVER}/f/privacy", 4)
        sw = wait_for(m, "const s=[...document.querySelectorAll('[role=switch]')].map(x=>x.getAttribute('aria-label')); return s.length?s:null") or []
        for want in ["Block trackers", "Block ads", "Refuse cookie pop-ups", "Clean tracking links", "More sites", "DuckDuckGo's protections too"]:
            check(who, f"Privacy has the switch “{want}”", want in sw, f"switches: {sw}")

        # Save media, with a page's embedded video.
        c = [{"url": "https://www.youtube.com/watch?v=AbCdEf12345", "kind": "player", "label": "YouTube video: Leafcutters"}]
        frag = json.dumps({"page": "https://news.example/ants", "c": c})
        goto(m, f"{SERVER}/f/media#{urllib.request.quote(frag)}", 4)
        listed = wait_for(m, "return document.body.innerText.includes('YouTube video: Leafcutters')")
        check(who, "Save media lists the page's embedded video", bool(listed), content(m, "return document.body.innerText")[:200])
    finally:
        m.cmd("WebDriver:DeleteSession")
        bc.stop(ff)


def tabs_now(m) -> list:
    return chrome(m, "return window.gBrowser.tabs.map(t => t.linkedBrowser.currentURI.spec)")


def opening_on_a_second_day(ff: Path) -> None:
    """Opened as the launcher opens it (through the server, on formicaria's own profile), closed,
    and opened again the next day: how long until the notes show, and how many tabs of them."""
    who = "opening formicaria, then again the next day"
    for day in ("first", "second"):
        t0 = time.time()
        bc.post("/api/browser_open")
        m = bc.Marionette()
        m.cmd("WebDriver:NewSession", {"capabilities": {}})
        seen = []
        for _ in range(120):
            seen = tabs_now(m)
            notes = [u for u in seen if "/f/group/Library" in u or u.rstrip("/") == SERVER]
            if notes:
                break
            time.sleep(0.25)
        took = time.time() - t0
        time.sleep(6)  # what else opens by itself, does so by now
        seen = tabs_now(m)
        notes = [u for u in seen if "/f/group/Library" in u or u.rstrip("/") == SERVER]
        check(who, f"{day} day: the notes show within 5 s", took < 5, f"{took:.1f} s")
        print(f"usability: [{who}] {day} day: notes in {took:.1f} s; tabs {seen}")
        check(who, f"{day} day: one tab of the notes, nothing unasked", len(notes) == 1 and len(seen) == 1, f"tabs: {seen}")
        try:
            m.cmd("Marionette:Quit", {"flags": ["eAttemptQuit"]})
        except Exception:  # noqa: BLE001 - the connection goes with the browser
            pass
        for _ in range(60):
            if not bc.procs_under(ff):
                break
            time.sleep(0.5)
        bc.stop(ff)


def left_on_settings(ff: Path) -> None:
    """**A session that ended on Settings alone** (2026-11-21, on a laptop: every start showed
    Settings alone, the notes never opened, and closing Settings closed the last tab, which ends the
    program). Left so, opened again: the notes must open beside it, and closing Settings must leave
    them open."""
    who = "someone whose last session ended on Settings alone"
    is_notes = lambda u: "/f/group/" in u or u.rstrip("/") == SERVER  # noqa: E731
    settings = f"{SERVER}/f/settings"

    def quit_browser(m) -> None:
        try:
            m.cmd("Marionette:Quit", {"flags": ["eAttemptQuit"]})
        except Exception:  # noqa: BLE001 - the connection goes with the browser
            pass
        for _ in range(60):
            if not bc.procs_under(ff):
                break
            time.sleep(0.5)
        bc.stop(ff)

    bc.post("/api/browser_open")
    m = bc.Marionette()
    m.cmd("WebDriver:NewSession", {"capabilities": {}})
    for _ in range(80):
        if tabs_now(m):
            break
        time.sleep(0.25)
    # After the start's own look (four seconds in), so it cannot add a tab to the one being set up.
    time.sleep(6)
    chrome(m, """for (const t of [...gBrowser.tabs]) if (t !== gBrowser.selectedTab) gBrowser.removeTab(t);
                 return true;""")
    goto(m, settings, 3)
    left = tabs_now(m)
    check(who, "set up: one tab, Settings", left == [settings], f"tabs: {left}")
    quit_browser(m)

    bc.post("/api/browser_open")
    m = bc.Marionette()
    m.cmd("WebDriver:NewSession", {"capabilities": {}})
    seen: list = []
    for _ in range(60):  # the startup script looks four seconds after the session came back
        seen = tabs_now(m)
        if any(is_notes(u) for u in seen):
            break
        time.sleep(0.25)
    check(who, "the next start opens the notes beside the Settings it left",
          any(is_notes(u) for u in seen) and settings in seen, f"tabs: {seen}")
    # Settings closed as a person closes a tab.
    chrome(m, f"""const t = [...gBrowser.tabs].find((x) => x.linkedBrowser.currentURI.spec === {json.dumps(settings)});
                  if (t) gBrowser.removeTab(t); return !!t;""")
    time.sleep(2)
    after = tabs_now(m) if bc.procs_under(ff) else []
    check(who, "closing Settings leaves the notes open, and the program with them",
          any(is_notes(u) for u in after) and settings not in after, f"tabs: {after}")
    quit_browser(m)


def update_bar(ff: Path) -> None:
    """**A newer formicaria, offered above every tab** (2026-11-21: a person on Settings alone never
    saw the notes' update line). The browser's own bar, handed what the server says when a newer
    version can be installed here: it shows over a web page or Settings, *Update* brings a tab of the
    notes forward and starts the one press there, *Later* puts it away for that version."""
    who = "someone on a page that is not the notes, with an update out"
    bc.post("/api/browser_open")
    m = bc.Marionette()
    m.cmd("WebDriver:NewSession", {"capabilities": {}})
    for _ in range(80):
        if tabs_now(m):
            break
        time.sleep(0.25)
    goto(m, f"{SERVER}/f/settings", 3)
    offer = {"available": "v99.0.0", "current": "v0.60.3", "can_install": True}
    probe = """
      const box = window.gNotificationBox;
      const n = box && box.getNotificationWithValue('formicaria-update');
      if (!n) return null;
      const all = [...n.querySelectorAll('button'), ...(n.shadowRoot ? n.shadowRoot.querySelectorAll('button') : [])];
      return { text: (n.messageText && n.messageText.textContent) || n.getAttribute('message') || n.textContent || '',
               buttons: all.map(b => (b.label || b.textContent || '').trim()).filter(Boolean) };"""
    chrome(m, "Services.prefs.clearUserPref('formicaria.update.later'); "
              f"return window.formicariaUpdateShow({json.dumps(offer)}).then(() => true)")
    bar = chrome(m, probe)
    check(who, "the browser's bar offers it over Settings", bool(bar) and "v99.0.0" in json.dumps(bar), str(bar))
    check(who, "with Update and Later", bool(bar) and {"Update", "Later"} <= set(bar.get("buttons", [])), str(bar))
    # Update, pressed as a person presses it: the notes come forward and take the one press.
    pressed = chrome(m, """
      const n = window.gNotificationBox.getNotificationWithValue('formicaria-update');
      const all = [...n.querySelectorAll('button'), ...(n.shadowRoot ? n.shadowRoot.querySelectorAll('button') : [])];
      const b = all.find(x => (x.label || x.textContent || '').trim() === 'Update');
      if (!b) return 'no Update button';
      b.click(); return 'pressed';""")
    shown = ""
    for _ in range(40):
        shown = chrome(m, "return window.gBrowser.selectedBrowser.currentURI.spec")
        if "/f/group/" in shown and "#update" not in shown:
            break
        time.sleep(0.25)
    check(who, "Update brings the notes forward, and the page takes the press",
          pressed == "pressed" and "/f/group/" in shown and "#update" not in shown, f"{pressed}; shown {shown}")
    # Later: away for that version.
    chrome(m, f"return window.formicariaUpdateShow({json.dumps(offer)}).then(() => true)")
    later = chrome(m, """
      const n = window.gNotificationBox.getNotificationWithValue('formicaria-update');
      if (!n) return 'no bar';
      const all = [...n.querySelectorAll('button'), ...(n.shadowRoot ? n.shadowRoot.querySelectorAll('button') : [])];
      const b = all.find(x => (x.label || x.textContent || '').trim() === 'Later');
      if (!b) return 'no Later button';
      b.click(); return 'pressed';""")
    time.sleep(1)
    chrome(m, f"return window.formicariaUpdateShow({json.dumps(offer)}).then(() => true)")
    again = chrome(m, probe)
    check(who, "Later puts it away for that version", later == "pressed" and not again, f"{later}; {again}")
    chrome(m, "Services.prefs.clearUserPref('formicaria.update.later'); return true")
    try:
        m.cmd("Marionette:Quit", {"flags": ["eAttemptQuit"]})
    except Exception:  # noqa: BLE001 - the connection goes with the browser
        pass
    for _ in range(60):
        if not bc.procs_under(ff):
            break
        time.sleep(0.5)
    bc.stop(ff)


def storage_cleanup(ff: Path, work: Path) -> None:
    """**Unused attachments, put aside and put back** (the owner, 2026-11-21: *"a broad clean up option
    in settings to allow the user to remove assets that nobody is using"*). A file whose only
    reference is its own attachment entry: found in Settings → Vaults & backup, ticked, removed from
    the vault's files, then put back."""
    who = "someone freeing the space of files no note uses"
    data = b"ID3\x04\x00\x00 a track nobody keeps" + bytes(4000)
    req = urllib.request.Request(f"{SERVER}/api/ingest?name=old-track.mp3&vault=", data=data, method="POST",
                                 headers={"Content-Type": "application/octet-stream"})
    asset = json.load(urllib.request.urlopen(req, timeout=30))
    h = asset["assets"][0].split(":", 1)[1]
    blob = work / "vault" / "blobs" / "sha256" / h[:2] / h[2:4] / h
    held = work / "vault" / ".fm-held" / h
    check(who, "set up: the file is in the vault", blob.exists(), str(blob))
    m = launch(ff, work / "profile-storage", f"{SERVER}/f/settings")
    try:
        time.sleep(6)
        content(m, """const n = document.querySelector('nav[aria-label="settings categories"]');
                      const b = n && [...n.querySelectorAll('button, a')].find((e) => e.textContent.trim() === 'Vaults & backup');
                      if (b) b.click(); return !!b;""")
        group = wait_for(m, "return !!document.querySelector('[aria-label^=\"unused attachments in \"]')", 20)
        check(who, "Vaults & backup has the unused attachments", bool(group))
        content(m, """const b = [...document.querySelectorAll('button')].find((e) => e.textContent.trim() === 'Look for attachments no note uses…');
                      if (b) b.click(); return !!b;""")
        row = wait_for(m, """const l = [...document.querySelectorAll('.storage-list label')].find((e) => e.textContent.includes('old-track.mp3'));
                             return l ? { ticked: l.querySelector('input').checked } : null;""", 20)
        check(who, "it is listed, kept only by its attachment entry, not ticked for the person",
              bool(row) and row.get("ticked") is False, str(row))
        content(m, """const l = [...document.querySelectorAll('.storage-list label')].find((e) => e.textContent.includes('old-track.mp3'));
                      l.querySelector('input').click();
                      setTimeout(() => [...document.querySelectorAll('button')].find((e) => e.textContent.trim().startsWith('Remove selected'))?.click(), 300);
                      return true;""")
        section = "(document.querySelector('[aria-label^=\"unused attachments in \"]') || {}).innerText || ''"
        said = wait_for(m, f"return /Put aside 1 file/.test({section})", 20)
        shown = content(m, "const g = document.querySelector('[aria-label^=\"unused attachments in \"]'); return g ? g.innerText.slice(0, 400) : 'no section: ' + location.href")
        check(who, "Remove selected puts it aside: out of the vault's files, held", bool(said) and not blob.exists() and held.exists(),
              f"said {said}; in blobs {blob.exists()}; held {held.exists()}; the section: {shown!r}")
        content(m, """[...document.querySelectorAll('button')].find((e) => e.textContent.trim() === 'Put them back')?.click(); return true;""")
        back = wait_for(m, f"return /Put back 1 file/.test({section})", 20)
        check(who, "Put them back returns it", bool(back) and blob.exists() and not held.exists(),
              f"said {back}; in blobs {blob.exists()}")
        m.cmd("Marionette:Quit", {"flags": ["eAttemptQuit"]})
    except Exception as e:  # noqa: BLE001 - the browser gone takes the connection with it
        check(who, "the browser still open", False, repr(e)[:200])
    bc.stop(ff)


READ_MEASURE_JS = r"""
const read = document.querySelector('.read');
const p = read && read.querySelector('p');
const pane = document.querySelector('article.panel');
if (!p || !pane) return null;
const probe = document.createElement('span');
probe.style.cssText = 'display:inline-block;width:1ch;visibility:hidden;position:absolute';
p.appendChild(probe);
const ch = probe.getBoundingClientRect().width;
probe.remove();
const pr = p.getBoundingClientRect();
const t = read.querySelector(':scope > table');
const tr = t ? t.getBoundingClientRect() : null;
return {
  chars: Math.round(pr.width / ch), text: Math.round(pr.width), pane: pane.clientWidth,
  table: tr ? Math.round(tr.width) : 0, tableMid: tr ? Math.round(tr.left + tr.width / 2) : 0,
  textMid: Math.round(pr.left + pr.width / 2), window: innerWidth,
};
"""


def reading_width(ff: Path, work: Path) -> None:
    """**Reading width** (the owner, 2026-11-21: a note on a laptop read *"super short in width"*).
    On a 1920 × 1080 window: the text runs about 75 characters by default; Settings → Look → Reading
    width, chosen in its own tab, changes the open note at once; ＋ → Full width widens one note; a
    wide table grows past the text, centred; the editor keeps the width the note reads at."""
    who = "someone reading on a wide laptop screen"
    words = " ".join(["The quick brown fox jumps over the lazy dog, then naps."] * 30)
    head = "| " + " | ".join(f"Column number {i}" for i in range(14)) + " |"
    rule = "|" + "---|" * 14
    row = "| " + " | ".join(f"value {i}" for i in range(14)) + " |"
    made = bc.post("/api/create_notes", {"notes": [{"title": "Wide reading", "body": f"{words}\n\n{head}\n{rule}\n{row}\n", "props": {}}]})
    nid = made[0]["id"]
    m = launch(ff, work / "profile-width", f"{SERVER}/f/note/{nid}")
    try:
        m.cmd("WebDriver:SetWindowRect", {"width": 1920, "height": 1080})
        time.sleep(6)
        got = wait_for(m, READ_MEASURE_JS, 20) or {}
        check(who, "by default a line holds about 75 characters (it was about 61)",
              70 <= (got.get("chars") or 0) <= 80, str(got))
        check(who, "a wide table grows past the text, up to the note, centred on the text",
              got.get("table", 0) > got.get("text", 0) + 50 and got.get("table", 0) <= got.get("pane", 0)
              and abs(got.get("tableMid", 0) - got.get("textMid", 0)) <= 2, str(got))
        def val(r):  # Marionette answers some commands bare, some under "value"
            return r.get("value", r) if isinstance(r, dict) else r
        note_win = val(m.cmd("WebDriver:GetWindowHandle"))
        win = val(m.cmd("WebDriver:NewWindow", {"type": "tab"}))
        m.cmd("WebDriver:SwitchToWindow", {"handle": win["handle"] if isinstance(win, dict) else win})
        m.cmd("WebDriver:Navigate", {"url": f"{SERVER}/f/settings"})
        picked = wait_for(m, """const l = document.querySelector('[aria-label="reading width"]');
                                const w = l && [...l.querySelectorAll('label')].find((x) => x.querySelector('.k')?.textContent.trim() === 'Wide');
                                if (w) w.querySelector('input').click(); return !!w;""", 25)
        check(who, "Settings → Look has Reading width", bool(picked))
        m.cmd("WebDriver:SwitchToWindow", {"handle": note_win})
        wide = wait_for(m, f"const r = (() => {{ {READ_MEASURE_JS} }})(); return r && r.chars >= 90 ? r : null;", 10) or {}
        check(who, "Wide, chosen in the Settings tab, reaches the open note at once: about 100 characters",
              92 <= (wide.get("chars") or 0) <= 108, str(wide))
        content(m, "document.querySelector('button[aria-label=\"note options\"]').click(); return true;")
        content(m, """const b = [...document.querySelectorAll('button.opt')].find((x) => x.textContent.trim() === 'Full width');
                      if (b) b.click(); return !!b;""", 600)
        full = wait_for(m, f"const r = (() => {{ {READ_MEASURE_JS} }})(); return r && r.text >= r.pane - 80 ? r : null;", 10) or {}
        check(who, "＋ → Full width gives this note the whole width", bool(full), str(full))
        content(m, "document.querySelector('button[aria-label=\"note options\"]').click(); return true;")
        content(m, """const b = [...document.querySelectorAll('button.opt')].find((x) => x.textContent.trim() === 'Readable width');
                      if (b) b.click(); return !!b;""", 600)
        back = wait_for(m, f"const r = (() => {{ {READ_MEASURE_JS} }})(); return r && r.chars <= 108 ? r : null;", 10) or {}
        check(who, "Readable width puts it back to the setting", 92 <= (back.get("chars") or 0) <= 108, str(back))
        reading = back.get("text", 0)
        content(m, "document.querySelector('.read p').click(); return true;", 300)
        ed = wait_for(m, """const c = document.querySelector('.cm-content');
                            if (!c) return null; const s = getComputedStyle(c);
                            return c.getBoundingClientRect().width - parseFloat(s.paddingLeft) - parseFloat(s.paddingRight);""", 10)
        check(who, "being written, the text keeps the width it reads at", bool(ed) and abs(ed - reading) <= 24,
              f"editor {ed} vs reading {reading}")
        m.cmd("Marionette:Quit", {"flags": ["eAttemptQuit"]})
    except Exception as e:  # noqa: BLE001 - the browser gone takes the connection with it
        check(who, "the browser still open", False, repr(e)[:200])
    bc.stop(ff)


def tone(seconds: int, hz: int) -> bytes:
    """A plain WAV of a tone, `seconds` long: a song every browser plays, made here, no download."""
    import io
    import math
    import struct
    import wave
    buf = io.BytesIO()
    with wave.open(buf, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(8000)
        w.writeframes(b"".join(struct.pack("<h", int(8000 * math.sin(2 * math.pi * hz * i / 8000)))
                               for i in range(8000 * seconds)))
    return buf.getvalue()


def music_player(ff: Path, work: Path) -> None:
    """**The music player, every button, and songs one after another** (the owner, 2026-11-21). Three
    20-second tones as songs in a playlist; Play pressed as a mouse presses it; then `ci/music-tour.js`
    goes through the deck as a person does and says what each button did."""
    who = "someone listening to a playlist"
    ids = []
    for name, hz in [("Tone A", 330), ("Tone B", 440), ("Tone C", 550)]:
        req = urllib.request.Request(f"{SERVER}/api/ingest?name={name.replace(' ', '-').lower()}.wav&vault=",
                                     data=tone(20, hz), method="POST",
                                     headers={"Content-Type": "application/octet-stream"})
        ref = json.load(urllib.request.urlopen(req, timeout=30))["assets"][0]
        made = bc.post("/api/create_notes", {"notes": [{"title": name, "props": {"log": "music", "audio": ref, "duration_s": 20}}]})
        ids.append(made[0]["id"])
    body = "".join(f"{i + 1}. [{t}](note:{n})\n" for i, (t, n) in enumerate(zip(["Tone A", "Tone B", "Tone C"], ids)))
    bc.post("/api/create_notes", {"notes": [{"title": "Three tones", "tags": ["playlist"], "body": body, "props": {}}]})
    m = launch(ff, work / "profile-music", f"{SERVER}/f/group/Music")
    try:
        time.sleep(6)
        content(m, """const t = [...document.querySelectorAll('[role=tab]')].find((b) => b.textContent.trim() === 'Playlists');
                      if (t) t.click(); return !!t;""")
        opened = wait_for(m, """const b = document.querySelector('button[aria-label="playlist Three tones"]');
                                if (b) b.click(); return !!b;""", 20)
        check(who, "the playlist is in Music", bool(opened))
        time.sleep(2)
        el = m.cmd("WebDriver:FindElement", {"using": "xpath", "value": "//button[normalize-space()='Play']"})["value"]
        m.cmd("WebDriver:ElementClick", {"id": next(iter(el.values()))})
        m.cmd("WebDriver:SetTimeouts", {"script": 600000})
        m.s.settimeout(660)  # the tour takes minutes; the connection's own 30 s would give up first
        tour = (ROOT / "ci" / "music-tour.js").read_text()
        r = m.cmd("WebDriver:ExecuteScript", {"args": [["Tone A", "Tone B", "Tone C"]],
                                              "script": f"return ({tour})(arguments[0]);"})
        got = r["value"] if isinstance(r, dict) else r
        m.s.settimeout(30)
        for name, st in (got or {}).get("steps", {}).items():
            check(who, name, st.get("ok"), st.get("said", ""))
        if not (got or {}).get("steps"):
            check(who, "the player could be gone through", False, str(got)[:300])
        m.cmd("Marionette:Quit", {"flags": ["eAttemptQuit"]})
    except Exception as e:  # noqa: BLE001 - the browser gone takes the connection with it
        check(who, "the browser still open", False, repr(e)[:200])
    bc.stop(ff)


PRIVACY_PORT = 8794


def privacy_minded(ff: Path, work: Path) -> None:
    """**Someone who counts on the protections** (`privacy-plan-2026-11-19.md` 1.2): the privacy test
    site (`ci/privacy_site.py`) opened in formicaria's browser as it comes, and what each protection
    actually did there read back: trackers refused, the page's own script run, the "do not sell"
    signal sent, tracking parts taken off a link to another site, the strict mode's fingerprinting and
    bounce-tracking protection on, lookups encrypted to the chosen resolver, and *More sites* bringing
    no extension."""
    who = "someone who counts on the protections"
    site = subprocess.Popen([sys.executable, str(ROOT / "ci" / "privacy_site.py"), str(PRIVACY_PORT)],
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    page = f"http://127.0.0.1:{PRIVACY_PORT}/page.html?other=http://localhost:{PRIVACY_PORT}"
    try:
        m = launch(ff, work / "profile-privacy", f"{SERVER}/f/group/Library")
        applied = None
        for _ in range(40):
            applied = chrome(m, "return Services.prefs.getIntPref('formicaria.privacy.v', -1)")
            if applied is not None and applied >= 0:
                break
            time.sleep(0.5)
        check(who, "the privacy switches reach the browser", applied is not None and applied >= 0, f"v={applied}")
        # **Whoever refuses them, and once the lists are there** (2026-11-19). A new profile reads its
        # tracker lists from Mozilla's list service after its first start, and with uBlock Origin Lite
        # downloading at the same first start a fixed 10 s sometimes read the page before them (nothing
        # refused; Firefox's "last update" preference proved no better a signal). So the page is opened
        # until the trackers are refused, for up to a minute. A tracker counts as refused when Firefox
        # recorded it refused or stood in for (its blocking log), or when the page asked for it and no
        # answer from its server came back (Lite, which answers before Firefox's protection sees it).
        chrome(m, """
          const net = window.__fmNet = {asked: [], answered: []};
          const host = (s) => { try { return s.QueryInterface(Ci.nsIChannel).URI.host; } catch (e) { return ''; } };
          const obs = window.__fmNetObs = { observe(s, t) {
            (t === 'http-on-opening-request' ? net.asked : net.answered).push(host(s)); } };
          for (const t of ['http-on-opening-request', 'http-on-examine-response',
                           'http-on-examine-cached-response', 'http-on-examine-merged-response'])
            Services.obs.addObserver(obs, t);
          return true;""")

        def blocking_log() -> set:
            m.cmd("Marionette:SetContext", {"value": "chrome"})
            try:
                log = m.cmd("WebDriver:ExecuteAsyncScript", {"args": [], "script": """
                  const done = arguments[arguments.length - 1];
                  Promise.resolve(gBrowser.selectedBrowser.getContentBlockingLog()).then(done, (e) => done(String(e)));"""})["value"]
            finally:
                m.cmd("Marionette:SetContext", {"value": "content"})
            try:
                entries = json.loads(log) if isinstance(log, str) else {}
            except ValueError:
                entries = {}
            # STATE_BLOCKED_TRACKING_CONTENT 0x1000, …SOCIALTRACKING 0x10000, …FINGERPRINTING 0x40,
            # STATE_REPLACED_TRACKING_CONTENT (a stand-in) 0x10, STATE_ALLOWED_TRACKING_CONTENT… not counted.
            return {o for o, rows in entries.items() for r in rows if r and r[1] and (r[0] & (0x1000 | 0x10000 | 0x40 | 0x10))}

        time.sleep(10)
        for attempt in range(1, 7):
            chrome(m, "window.__fmNet.asked.length = 0; window.__fmNet.answered.length = 0; return true;")
            goto(m, page, 6)
            net = chrome(m, "return window.__fmNet") or {}
            refused = blocking_log()

            def kept_out(part: str) -> bool:
                asked = any(part in h for h in net.get("asked", []))
                answered = any(part in h for h in net.get("answered", []))
                return any(part in o for o in refused) or (asked and not answered)
            ga, fb = kept_out("google-analytics.com"), kept_out("facebook.net")
            if ga and fb:
                break
            time.sleep(4)
        exts = chrome(m, "return JSON.parse(Services.prefs.getStringPref('formicaria.privacy.addons', '[]'))")
        fm = content(m, "return window.fm || null") or {}
        check(who, "the page's own script runs", fm.get("app") == 1, json.dumps(fm)[:300])
        trackers = lambda xs: sorted(h for h in set(xs) if "google" in h or "facebook" in h)
        print(f"usability:      trackers, opening {attempt}: Firefox refused {sorted(refused)}; asked "
              f"{trackers(net.get('asked', []))}, answered {trackers(net.get('answered', []))}; extensions {exts}")
        check(who, "trackers are refused (Facebook's pixel; Google Analytics stood in for)", ga and fb,
              f"after {attempt} openings: Firefox refused {sorted(refused)}; answered "
              f"{trackers(net.get('answered', []))}; extensions {exts}; "
              f"page {json.dumps({k: fm.get(k) for k in ('ga', 'fb')})}")
        check(who, "sites are told not to sell or share (Sec-GPC: 1)",
              (fm.get("headers") or {}).get("sec-gpc") == "1", json.dumps(fm.get("headers"))[:300])
        content(m, "document.getElementById('tracked').click(); return true;")
        time.sleep(4)
        landed = content(m, "return location.href") or ""
        check(who, "a link to another site loses its tracking parts", "landing.html" in landed and "id=7" in landed
              and not any(t in landed for t in ("fbclid", "utm_source", "gclid")), landed)
        prefs = chrome(m, """return {
          category: Services.prefs.getStringPref('browser.contentblocking.category', ''),
          strict: Services.prefs.getStringPref('browser.contentblocking.features.strict', ''),
          trr: Services.prefs.getIntPref('network.trr.mode', 0),
          uri: Services.prefs.getStringPref('network.trr.uri', ''),
          strip: Services.prefs.getStringPref('privacy.query_stripping.strip_list', '').split(' ').length,
        }""") or {}
        feats = set(str(prefs.get("strict", "")).split(","))
        check(who, "strict protection, with fingerprinting and bounce-tracking protection",
              prefs.get("category") == "strict" and {"fpp", "btp", "qps", "fp"} <= feats, json.dumps(prefs)[:300])
        check(who, "lookups go encrypted to Quad9 (the system's only as fallback)",
              prefs.get("trr") == 2 and "quad9" in str(prefs.get("uri")), json.dumps(prefs)[:300])
        check(who, "Firefox strips the wider list of tracking parts", (prefs.get("strip") or 0) > 200, json.dumps(prefs)[:300])
        bc.post("/api/set_privacy", {"more_sites": True})
        time.sleep(12)
        addons = chrome(m, "return Services.prefs.getStringPref('formicaria.privacy.addons', '[]')") or "[]"
        check(who, "More sites brings no extension", "gdpr@cavi.au.dk" not in addons and "74145f27" not in addons, addons)
        bc.post("/api/set_privacy", {"more_sites": False})
        try:
            m.cmd("Marionette:Quit", {"flags": ["eAttemptQuit"]})
        except Exception:  # noqa: BLE001 - the connection goes with the browser
            pass
        bc.stop(ff)
    finally:
        site.terminate()


def settings_alone(ff: Path, work: Path) -> None:
    """**Settings, the only tab, clicked around** (2026-11-19; the owner: *"the update lines were out
    but not clickable, as I tried to click on a random place it crashed"*). formicaria's Settings page
    opened on its own (one tab, one entry in its history, as a start restores it); a click beside the
    panel, a click where the update notice is, then its ✕: the browser still open after each, and
    leaving Settings lands on the notes."""
    who = "someone on the Settings page alone"
    m = launch(ff, work / "profile-settings", f"{SERVER}/f/settings")
    time.sleep(6)
    alive = lambda: chrome(m, "return gBrowser.tabs.length")  # noqa: E731 - raises once the browser has gone

    def click_at(x: int, y: int) -> None:
        m.cmd("WebDriver:PerformActions", {"actions": [{
            "type": "pointer", "id": "mouse", "parameters": {"pointerType": "mouse"},
            "actions": [{"type": "pointerMove", "x": x, "y": y, "origin": "viewport"},
                        {"type": "pointerDown", "button": 0}, {"type": "pointerUp", "button": 0}]}]})
        m.cmd("WebDriver:ReleaseActions", {})
        time.sleep(2)

    try:
        # One tab, Settings: any other the start opened is closed first — the notes too, which the
        # start now opens beside a Settings that came back alone (`left_on_settings`, 2026-11-21).
        chrome(m, """const keep = [...gBrowser.tabs].find((t) => t.linkedBrowser.currentURI.spec.includes('/f/settings'))
                       || gBrowser.selectedTab;
                     gBrowser.selectedTab = keep;
                     for (const t of [...gBrowser.tabs]) if (t !== keep) gBrowser.removeTab(t);
                     return true;""")
        shown = wait_for(m, "return !!document.querySelector('[role=dialog], .panel')", 20)
        check(who, "Settings shows on its own page", bool(shown))
        check(who, "nothing covers the page to close Settings on a click",
              content(m, "return document.querySelector('.settings-backdrop') === null") is True)
        click_at(12, 400)   # beside the panel
        click_at(200, 12)   # the top line, where the update notice is
        still = alive()
        check(who, "a click beside Settings leaves the browser open", bool(still), f"tabs {still}")
        content(m, "document.querySelector('button[aria-label=close]')?.click(); return true;")
        time.sleep(3)
        still = alive()
        landed = content(m, "return location.pathname") or ""
        check(who, "closing Settings goes to the notes, the browser still open",
              bool(still) and landed.startswith("/f/group/Library"), f"tabs {still}, at {landed}")
        m.cmd("Marionette:Quit", {"flags": ["eAttemptQuit"]})
    except Exception as e:  # noqa: BLE001 - the browser gone takes the connection with it
        # Said, and nothing more asked of a connection whose browser has gone (asking hung).
        check(who, "the browser still open", False, repr(e)[:200])
    bc.stop(ff)


def diary_kept(work: Path) -> None:
    """**The usage diary kept this run** (`fm_app::diary`, 2026-11-20): the server's start, commands
    counted by name, a sample of each part's memory, the browser's start and the pages opened — and
    no web address and no note's title or text, though this run opened sites and wrote notes."""
    who = "the usage diary"
    time.sleep(16)  # one more turn of the sampler
    lines = []
    for f in sorted((work / "config" / "formicaria" / "diary").glob("*.jsonl")):
        for l in f.read_text().splitlines():
            try:
                lines.append(json.loads(l))
            except ValueError:
                pass
    kinds = {v.get("kind") for v in lines}
    check(who, "this start is in it", "start" in kinds, str(sorted(kinds, key=str)))
    used = {c for v in lines if v.get("kind") == "use" for c in v.get("commands", {})}
    check(who, "commands are counted by name", len(used) >= 3, str(sorted(used))[:300])
    parts = {p for v in lines if v.get("kind") == "sample" for p in v.get("parts", {})}
    check(who, "each part's memory is sampled", {"server", "browser"} <= parts, str(sorted(parts)))
    whats = {v.get("what") for v in lines if v.get("kind") == "browser"}
    check(who, "formicaria's browser says its start", "started" in whats, str(sorted(whats, key=str)))
    pages = {v.get("page") for v in lines if v.get("kind") == "page"}
    check(who, "pages opened are named by their kind", bool(pages), str(sorted(pages, key=str)))
    text = "\n".join(json.dumps(v) for v in lines)
    leaks = [w for w in ("://", "wikipedia", "youtube", "Check Reader", "reader@example.org") if w in text]
    check(who, "no address, name or note in it", not leaks, str(leaks))


def look(ff: Path, work: Path) -> None:
    """**What a person sees, as pictures** (the owner, 2026-11-19: *"see it yourself"*): formicaria's
    browser on a video page, the whole window drawn to `work/look-*.png`, and where formicaria's own
    buttons are, measured. Run on its own: `FM_USABILITY_ONLY=look`."""
    who = "what a person sees"
    m = launch(ff, work / "profile-look", "https://www.youtube.com/watch?v=Cr8K88UcO0s", 'user_pref("browser.window.width", 1400);\nuser_pref("browser.window.height", 900);\n')
    time.sleep(10)
    m.cmd("Marionette:SetContext", {"value": "chrome"})
    try:
        # The video's tab in front, as the person had it (formicaria opens its notes beside it).
        m.cmd("WebDriver:ExecuteScript", {"args": [], "script": """
          const t = gBrowser.tabs.find((t) => /youtube/.test(t.linkedBrowser.currentURI.spec));
          if (t) gBrowser.selectedTab = t;"""})
        time.sleep(2)
        icons = m.cmd("WebDriver:ExecuteScript", {"args": [], "script": """
          const ant = document.getElementById('browser_formicaria-org_github_io-browser-action');
          const page = document.getElementById('pageAction-urlbar-browser_formicaria-org_github_io');
          const img = (e) => e && (getComputedStyle(e).listStyleImage + ' ' + (e.querySelector('image, .urlbar-icon, .toolbarbutton-icon')?.getAttribute('src') ?? ''));
          return { ant: img(ant), page: img(page) };"""})["value"]
        print(f"usability: [{who}] icons: {json.dumps(icons)[:400]}")
        where = m.cmd("WebDriver:ExecuteScript", {"args": [], "script": """
          const out = {};
          for (const id of ['browser_formicaria-org_github_io-browser-action', 'formicaria-settings', 'formicaria-home', 'pageActionButton']) {
            const e = document.getElementById(id);
            const r = e?.getBoundingClientRect();
            out[id] = e ? { w: Math.round(r.width), h: Math.round(r.height), x: Math.round(r.x), y: Math.round(r.y),
              hidden: e.hidden, display: getComputedStyle(e).display, parent: e.parentElement?.id } : null;
          }
          out.urlbarButtons = [...document.querySelectorAll('#page-action-buttons > *, .urlbar-page-action')].map((e) => e.id).filter(Boolean);
          out.tabsToolbar = [...(document.getElementById('TabsToolbar-customization-target')?.children ?? [])].map((e) => e.id);
          return out;"""})["value"]
        print(f"usability: [{who}] buttons: {json.dumps(where)}")
        shot = m.cmd("WebDriver:TakeScreenshot", {"full": False, "hash": False})["value"]
        (work / "look-window.png").write_bytes(base64.b64decode(shot))
    finally:
        m.cmd("Marionette:SetContext", {"value": "content"})
    ant = (where or {}).get("browser_formicaria-org_github_io-browser-action") or {}
    check(who, "the ant is drawn in the window", bool(ant) and ant.get("w", 0) > 0 and ant.get("h", 0) > 0, json.dumps(ant))
    try:
        m.cmd("Marionette:Quit", {"flags": ["eAttemptQuit"]})
    except Exception:  # noqa: BLE001
        pass
    bc.stop(ff)


VISIBLE_JS = r"""
const seen = (e) => { const r = e.getBoundingClientRect(); const st = getComputedStyle(e);
  return r.width > 0 && r.height > 0 && st.visibility !== 'hidden' && r.bottom > 0 && r.top < innerHeight; };
const name = (e) => (e.getAttribute('aria-label') || e.textContent || e.getAttribute('title') || '').trim().replace(/\s+/g, ' ').slice(0, 60);
return {
  url: location.pathname, title: document.title,
  controls: [...document.querySelectorAll('button, a[href], [role=tab], [role=menuitem], input, select')]
    .filter(seen).map((e) => `${e.tagName.toLowerCase()}${e.getAttribute('role') ? '[' + e.getAttribute('role') + ']' : ''}: ${name(e)}${e.title && e.title !== name(e) ? ' (title: ' + e.title + ')' : ''}`),
  headings: [...document.querySelectorAll('h1, h2, h3')].filter(seen).map(name),
};
"""


def first_hour_pictures(ff: Path, work: Path) -> None:
    """**The first hour, as pictures** (the owner, 2026-11-22: *"do the audit first"*): formicaria's
    browser at 1440 × 900 on the screens a new person meets, each drawn to `work/audit-*.png` with the
    controls a person can see listed in `work/audit-*.json`. Run on its own: `FM_USABILITY_ONLY=audit`.
    Invented notes only."""
    who = "the first hour, pictures"
    bc.post("/api/create_notes", {"notes": [
        {"title": "Reading list", "tags": ["reading"], "body": "- The ant colony as a superorganism\n- Trails and pheromones\n", "props": {}},
        {"title": "Field notes, week 3", "body": "## Tuesday\n\nThe colony moved two metres east. **Why?** See [[Reading list]].\n", "props": {}},
        {"title": "Write the methods section", "body": "", "props": {"due": "2026-11-28", "status": "todo"}},
        {"title": "Order the new nest", "body": "", "props": {"due": "2026-11-25", "status": "doing"}},
    ]})
    made = bc.post("/api/create_notes", {"notes": [{"title": "Meeting with the lab", "body": "Agenda:\n\n1. Results\n2. Next steps\n", "props": {}}]})
    nid = made[0]["id"]
    m = launch(ff, work / "profile-audit", f"{SERVER}/f/group/Library",
               'user_pref("browser.window.width", 1440);\nuser_pref("browser.window.height", 900);\n')
    def shot(name: str) -> None:
        time.sleep(2)
        try:
            (work / f"audit-{name}.json").write_text(json.dumps(content(m, VISIBLE_JS), indent=1))
        except Exception as e:  # noqa: BLE001
            (work / f"audit-{name}.json").write_text(json.dumps({"error": repr(e)}))
        m.cmd("Marionette:SetContext", {"value": "chrome"})
        try:
            png = m.cmd("WebDriver:TakeScreenshot", {"full": False, "hash": False})["value"]
            (work / f"audit-{name}.png").write_bytes(base64.b64decode(png))
        finally:
            m.cmd("Marionette:SetContext", {"value": "content"})
    def key(*keys: str) -> None:
        down = [{"type": "keyDown", "value": k} for k in keys]
        up = [{"type": "keyUp", "value": k} for k in reversed(keys)]
        m.cmd("WebDriver:PerformActions", {"actions": [{"type": "key", "id": "kb", "actions": down + up}]})
        m.cmd("WebDriver:ReleaseActions")
    try:
        m.cmd("WebDriver:SetWindowRect", {"width": 1440, "height": 900})
        time.sleep(8)
        shot("01-home")
        (work / "audit-preview.json").write_text(json.dumps(content(m, """
          return [...document.querySelectorAll('.post .preview, .preview')].slice(0, 4).map((p) => {
            const st = getComputedStyle(p); const r = p.getBoundingClientRect();
            return { text: p.textContent.slice(0, 120), h: Math.round(r.height), top: Math.round(r.top),
              display: st.display, maxH: st.maxHeight, overflow: st.overflow, color: st.color, mask: st.maskImage,
              ws: st.whiteSpace, lh: st.lineHeight, fs: st.fontSize, scrollH: p.scrollHeight, scrollTop: p.scrollTop,
              cls: p.className, parent: getComputedStyle(p.parentElement).display + ' ' + getComputedStyle(p.parentElement).flexDirection };
          });"""), indent=1))
        (work / "audit-preview-top.json").write_text(json.dumps(content(m, """
          const p = document.querySelector('.post .preview'); if (!p) return null;
          const r = p.getBoundingClientRect();
          const at = (y) => { const e = document.elementFromPoint(r.left + 15, y); return e ? e.tagName + '.' + e.className + ' ' + (e.textContent || '').slice(0, 30) : null; };
          return { line1: at(r.top + 6), line2: at(r.top + 25), line3: at(r.bottom - 6), rect: [r.left, r.top, r.width, r.height] };""")))
        content(m, "const s = document.createElement('style'); s.textContent = '.post .preview { mask-image: none !important; }'; document.head.appendChild(s); return true;")
        shot("01b-no-mask")
        goto(m, f"{SERVER}/f/new", 4); shot("02-new-tab")
        goto(m, f"{SERVER}/f/note/{nid}", 5); shot("03-note")
        content(m, "document.querySelector('button[aria-label=\"note options\"]')?.click(); return true;")
        shot("04-note-options")
        key("\ue00c")  # Escape
        content(m, "document.querySelector('.read p, .read li')?.click(); return true;")
        shot("05-note-editing")
        key("\ue00c")
        goto(m, f"{SERVER}/f/group/Library", 5)
        key("\ue009", "k"); shot("06-ctrl-k")  # Control+K
        key("\ue00c")
        key("/"); shot("07-slash-search")
        key("\ue00c")
        for path, name in [("/f/group/Notes", "08-notes"), ("/f/group/Tasks", "09-tasks"), ("/f/agenda", "10-agenda"),
                           ("/f/settings", "11-settings"), ("/f/privacy", "12-privacy")]:
            goto(m, f"{SERVER}{path}", 5)
            shot(name)
        check(who, "the pictures were drawn", (work / "audit-01-home.png").exists())
        m.cmd("Marionette:Quit", {"flags": ["eAttemptQuit"]})
    except Exception as e:  # noqa: BLE001
        check(who, "the browser still open", False, repr(e)[:200])
    bc.stop(ff)


AUDIT_JS = r"""
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const seen = (e) => { if (!e) return false; const r = e.getBoundingClientRect(); return r.width > 0 && r.height > 0; };
const byText = (re, sel = 'button, a, [role=menuitem], [role=tab], summary') =>
  [...document.querySelectorAll(sel)].filter(seen).find((e) => re.test((e.textContent || e.getAttribute('aria-label') || '').trim()));
const until = async (f, secs = 6) => { for (let i = 0; i < secs * 4; i++) { let v = null; try { v = f(); } catch {} if (v) return v; await sleep(250); } return null; };
const task = arguments[0];
const out = { task, ok: false, clicks: 0, said: '' };
const click = async (e) => { if (!e) return false; e.click(); out.clicks++; await sleep(700); return true; };
const key = (k, o = {}) => (document.activeElement || document.body).dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, ...o }));
try {
  if (task === 'search-visible') {
    const f = [...document.querySelectorAll('input')].filter(seen).find((i) => i.type === 'search' || /search|find/i.test(i.placeholder + (i.getAttribute('aria-label') || '')));
    out.ok = !!f; out.said = f ? (f.placeholder || f.getAttribute('aria-label')) : 'no search field on the screen';
  } else if (task === 'own-view') {
    const b = byText(/new view|add view|create view|\+ view|new board|new table/i);
    out.ok = !!b; out.said = b ? b.textContent.trim() : 'no control to make a view';
  } else if (task === 'history') {
    await click(document.querySelector('button[aria-label="note options"]'));
    const b = byText(/history|versions?|earlier/i, 'button.opt');
    out.ok = !!b; out.said = b ? b.textContent.trim() : 'note options: ' + [...document.querySelectorAll('button.opt')].map((e) => e.textContent.trim()).join(', ');
    document.querySelector('button[aria-label="close options"]')?.click();
  } else if (task === 'link-suggest') {
    document.querySelector('.read p, .read li')?.click(); out.clicks++;
    const ed = await until(() => document.querySelector('.cm-content'));
    if (!ed) { out.said = 'no editor'; return out; }
    ed.focus();
    document.execCommand('insertText', false, ' [[Rea');
    const pop = await until(() => [...document.querySelectorAll('[role=listbox], .slash-menu')].find(seen), 3);
    out.ok = !!pop; out.said = pop ? pop.textContent.trim().slice(0, 80) : 'typing [[ offers nothing';
  } else if (task === 'delete-undo') {
    await click(document.querySelector('button[aria-label="note options"]'));
    await click(byText(/^Delete$/, 'button.opt'));
    await click(byText(/^(Delete|Yes, delete|Delete it)$/));
    const toast = await until(() => [...document.querySelectorAll('[role=status], [role=alert], .toast')].find((e) => /undo/i.test(e.textContent)), 4);
    out.ok = !!toast; out.said = toast ? toast.textContent.trim().slice(0, 80) : 'after delete: ' + location.pathname + ' — no Undo offered';
  }
} catch (e) { out.said = 'error: ' + e; }
return out;
"""


def first_hour_basics(ff: Path, work: Path) -> None:
    """**What a computer had lost, back** (`ui-audit-2026-11-22.md`, U1): search from the Notes page;
    Ctrl+K and `/` as real key presses, opening formicaria's search and not the browser's own; a note
    deleted in its tab, offered back with Undo on the tab in front, and Ctrl+Z; Escape closing a
    note's Options; Recently deleted in the chooser; tab titles that name the page."""
    who = "someone in their first hour"
    made = bc.post("/api/create_notes", {"notes": [
        {"title": "Harvester ants", "body": "They store seeds.\n", "props": {}},
        {"title": "A note to throw away", "body": "Nothing much.\n", "props": {}}]})
    keep, drop = made[0]["id"], made[1]["id"]
    m = launch(ff, work / "profile-basics", f"{SERVER}/f/group/Library",
               'user_pref("browser.window.width", 1440);\nuser_pref("browser.window.height", 900);\n')
    def key(*keys: str) -> None:
        down = [{"type": "keyDown", "value": k} for k in keys]
        up = [{"type": "keyUp", "value": k} for k in reversed(keys)]
        m.cmd("WebDriver:PerformActions", {"actions": [{"type": "key", "id": "kb", "actions": down + up}]})
        m.cmd("WebDriver:ReleaseActions")
    def findbar_open() -> bool:
        return bool(chrome(m, "const f = gBrowser.getCachedFindBar && gBrowser.getCachedFindBar(); return !!(f && !f.hidden);"))
    try:
        time.sleep(7)
        # Search from the Notes page: the field is there, typing finds the note.
        typed = content(m, """const f = document.querySelector('input[aria-label="search notes"]');
                              if (!f) return false; f.focus(); return true;""")
        check(who, "the Notes page has a search field", bool(typed))
        if typed:
            m.cmd("WebDriver:PerformActions", {"actions": [{"type": "key", "id": "kb", "actions":
                  [a for ch in "harvester" for a in ({"type": "keyDown", "value": ch}, {"type": "keyUp", "value": ch})]}]})
            m.cmd("WebDriver:ReleaseActions")
            found = wait_for(m, "const r = document.querySelector('[aria-label=\"search results\"]'); return r && r.textContent.includes('Harvester ants') ? r.textContent.slice(0, 80) : null", 10)
            check(who, "typing in it finds the note", bool(found))
            key("\ue00c")
        # A post's text is seen (no mask hiding it): the pixel under its first line is the text's
        # colour somewhere, read from a picture of the page.
        seen_text = content(m, """const p = [...document.querySelectorAll('.post .preview')].find((e) => e.textContent.includes('They store seeds'));
                                  return p ? getComputedStyle(p).maskImage : 'no preview';""")
        check(who, "a post's text shows in Notes, not masked away", seen_text in ("none", ""), str(seen_text))
        # Ctrl+K and / on a note's page.
        goto(m, f"{SERVER}/f/note/{keep}", 5)
        content(m, "document.activeElement && document.activeElement.blur(); document.body.focus(); return true;")
        key("\ue009", "k"); time.sleep(1)
        box = content(m, "return !!document.querySelector('[role=dialog][aria-label=\"search notes\"]')")
        check(who, "Ctrl+K on a note's page opens the search", bool(box))
        key("\ue00c"); time.sleep(0.5)
        still_note = content(m, "return location.pathname")
        check(who, "Escape puts the search away and leaves the note open", still_note.startswith("/f/note/"), still_note)
        key("/"); time.sleep(1)
        box = content(m, "return !!document.querySelector('[role=dialog][aria-label=\"search notes\"]')")
        check(who, "/ opens formicaria's search, not the browser's find bar", bool(box) and not findbar_open(),
              f"search {box}, find bar {findbar_open()}")
        key("\ue00c"); time.sleep(0.5)
        # Escape closes a note's Options.
        content(m, "document.querySelector('button[aria-label=\"note options\"]').click(); return true;")
        time.sleep(0.7)
        key("\ue00c"); time.sleep(0.7)
        still = content(m, "return !!document.querySelector('.options-window')")
        check(who, "Escape closes a note's Options", not still)
        # Delete a note in its own tab; the Notes tab in front offers it back.
        notes_tab = m.cmd("WebDriver:GetWindowHandle")
        notes_tab = notes_tab.get("value", notes_tab) if isinstance(notes_tab, dict) else notes_tab
        goto(m, f"{SERVER}/f/group/Library", 5)
        win = m.cmd("WebDriver:NewWindow", {"type": "tab"})
        win = win.get("value", win) if isinstance(win, dict) else win
        m.cmd("WebDriver:SwitchToWindow", {"handle": win["handle"] if isinstance(win, dict) else win})
        m.cmd("WebDriver:Navigate", {"url": f"{SERVER}/f/note/{drop}"})
        time.sleep(4)
        content(m, """document.querySelector('button[aria-label="note options"]').click();
                      setTimeout(() => [...document.querySelectorAll('button.opt')].find((b) => b.textContent.trim() === 'Delete')?.click(), 500);
                      setTimeout(() => [...document.querySelectorAll('button.danger')].find((b) => b.textContent.trim() === 'Delete')?.click(), 1200);
                      return true;""")
        time.sleep(3)
        gone = bc.post("/api/get", {"id": drop})
        check(who, "set up: the note is deleted", not (gone or {}).get("id"), str(gone)[:120])
        m.cmd("WebDriver:SwitchToWindow", {"handle": notes_tab})
        toast = wait_for(m, "const t = [...document.querySelectorAll('[role=status]')].find((e) => e.textContent.includes('Deleted')); return t ? t.textContent : null", 8)
        check(who, "the tab in front offers it back: Deleted “…” · Undo", bool(toast) and "Undo" in (toast or ""), str(toast))
        content(m, "document.activeElement && document.activeElement.blur(); return true;")
        key("\ue009", "z"); time.sleep(3)
        back = (bc.post("/api/get", {"id": drop}) or {}).get("id") == drop
        check(who, "Ctrl+Z puts the note back", back)
        # Deleted in the same tab, after moving around in it: Back brings the page before out of the
        # browser's cache, and it says so too.
        goto(m, f"{SERVER}/f/note/{drop}", 4)
        content(m, """document.querySelector('button[aria-label="note options"]').click();
                      setTimeout(() => [...document.querySelectorAll('button.opt')].find((b) => b.textContent.trim() === 'Delete')?.click(), 500);
                      setTimeout(() => [...document.querySelectorAll('button.danger')].find((b) => b.textContent.trim() === 'Delete')?.click(), 1200);
                      return true;""")
        time.sleep(3)
        toast = wait_for(m, "const t = [...document.querySelectorAll('[role=status]')].find((e) => e.textContent.includes('Deleted')); return t ? location.pathname + ': ' + t.textContent : null", 8)
        check(who, "deleted in the same tab, the page it goes back to offers Undo", bool(toast), str(toast))
        content(m, "const b = [...document.querySelectorAll('[role=status] button')].find((e) => e.textContent.trim() === 'Undo'); if (b) b.click(); return !!b;")
        time.sleep(2)
        check(who, "its Undo puts the note back", (bc.post("/api/get", {"id": drop}) or {}).get("id") == drop)
        # Recently deleted, from the chooser.
        goto(m, f"{SERVER}/f/group/Library", 5)
        content(m, "document.querySelector('button[aria-label=\"switch this tab to another feature\"]').click(); return true;")
        time.sleep(0.6)
        listed = content(m, "return [...document.querySelectorAll('[role=menuitem]')].some((b) => b.textContent.trim() === 'Recently deleted…')")
        check(who, "Recently deleted is in the chooser", bool(listed))
        key("\ue00c")
        # Tab titles.
        for path, want in [("/f/new", "New tab"), ("/f/settings", "Settings")]:
            goto(m, f"{SERVER}{path}", 4)
            t = content(m, "return document.title")
            check(who, f"{path} is named “{want}” in its tab", t.startswith(want), t)
        m.cmd("Marionette:Quit", {"flags": ["eAttemptQuit"]})
    except Exception as e:  # noqa: BLE001
        check(who, "the browser still open", False, repr(e)[:200])
    bc.stop(ff)


def first_hour_walk(ff: Path, work: Path) -> None:
    """**The first hour, walked** (the audit, 2026-11-22): each task tried the way the mainstream
    apps do it, in formicaria's browser, and what happened written to `work/audit-walk.json`. On
    request only (`FM_USABILITY_ONLY=audit`): a gap here is a finding, not a failed release; a task
    fixed becomes an ordinary check."""
    made = bc.post("/api/create_notes", {"notes": [{"title": "Walk note", "body": "Some text to read.\n", "props": {}}]})
    nid = made[0]["id"]
    m = launch(ff, work / "profile-walk", f"{SERVER}/f/group/Library",
               'user_pref("browser.window.width", 1440);\nuser_pref("browser.window.height", 900);\n')
    walk = []
    def run(task: str) -> None:
        try:
            r = m.cmd("WebDriver:ExecuteAsyncScript", {"args": [task], "script":
                      "const done = arguments[arguments.length - 1]; (async function() {" + AUDIT_JS + "}).apply(null, arguments).then(done, (e) => done({task: arguments[0], ok: false, said: String(e)}));"})
            v = r["value"] if isinstance(r, dict) and "value" in r else r
        except Exception as e:  # noqa: BLE001
            v = {"task": task, "ok": False, "said": repr(e)[:200]}
        walk.append(v)
        print(f"usability: audit {'yes' if v.get('ok') else 'NO '} {task}: {v.get('said', '')} ({v.get('clicks', 0)} clicks)", flush=True)
    def key(*keys: str) -> None:
        down = [{"type": "keyDown", "value": k} for k in keys]
        up = [{"type": "keyUp", "value": k} for k in reversed(keys)]
        m.cmd("WebDriver:PerformActions", {"actions": [{"type": "key", "id": "kb", "actions": down + up}]})
        m.cmd("WebDriver:ReleaseActions")
    try:
        m.cmd("WebDriver:SetTimeouts", {"script": 60000})
        time.sleep(8)
        run("search-visible")
        run("own-view")
        # Ctrl+K and / as real key presses: what opens?
        key("\ue009", "k"); time.sleep(1.5)
        opened = content(m, "const d=[...document.querySelectorAll('[role=dialog], [role=combobox], [role=listbox]')].filter(e=>e.getBoundingClientRect().width>0).map(e=>e.getAttribute('aria-label')||e.className); const a=document.activeElement; if (a && a.getAttribute('aria-label')==='search notes') d.push('search notes (focused)'); return d.join(', ')")
        focus = chrome(m, "return document.commandDispatcher.focusedElement?.id || document.activeElement?.id || ''")
        walk.append({"task": "ctrl-k", "ok": bool(opened), "said": opened or f"nothing in the page; the window's focus went to {focus or 'the page'}"})
        print(f"usability: audit {'yes' if opened else 'NO '} ctrl-k: {walk[-1]['said']}", flush=True)
        key("\ue00c")
        goto(m, f"{SERVER}/f/note/{nid}", 5)
        run("history")
        run("link-suggest")
        key("\ue00c")
        goto(m, f"{SERVER}/f/note/{nid}", 5)
        run("delete-undo")
        key("\ue009", "z"); time.sleep(2)
        back = (bc.post("/api/get", {"id": nid}) or {}).get("id") == nid if True else False
        walk.append({"task": "ctrl-z-restores", "ok": back, "said": "the note came back" if back else "Ctrl+Z after deleting: the note stays deleted"})
        print(f"usability: audit {'yes' if back else 'NO '} ctrl-z-restores: {walk[-1]['said']}", flush=True)
        (work / "audit-walk.json").write_text(json.dumps(walk, indent=1))
        m.cmd("Marionette:Quit", {"flags": ["eAttemptQuit"]})
    except Exception as e:  # noqa: BLE001
        print(f"usability: audit walk stopped: {e!r}"[:300])
    bc.stop(ff)


def previous_release() -> str | None:
    """The newest release tag before this tree: what a person updating comes from."""
    r = subprocess.run(["git", "describe", "--tags", "--abbrev=0", "--match", "v*", "HEAD"], cwd=ROOT,
                       capture_output=True, text=True)
    return r.stdout.strip() or None


def after_an_update(ff: Path) -> None:
    """**Updated, and everything there at the first start** (2026-11-19; the owner, after 0.55.0's
    Settings was nowhere: *"One should update and have everything there automatically and in a
    reasonable time. Full stop."*). The browser holds the previous release's kit, as an update leaves
    it; formicaria opens it as the launcher does, and at that first start the kit must be this one,
    Settings in the tab row and working, and nothing left waiting for another restart."""
    who = "after an update, at the first start"
    bc.stop(ff)
    tag = previous_release()
    old = subprocess.run(["git", "show", f"{tag}:browser/kit/formicaria.cfg"], cwd=ROOT,
                         capture_output=True, text=True).stdout if tag else ""
    (ff / "formicaria.cfg").write_text(old or "// an older kit\n")
    (ff / "formicaria" / "kit-stamp").write_text("0000000000000000")
    check(who, "the browser starts on the previous release's kit", not bc.post("/api/browser_status").get("kit_current"),
          f"kit from {tag}")
    t0 = time.time()
    bc.post("/api/browser_open")
    m = bc.Marionette()
    m.cmd("WebDriver:NewSession", {"capabilities": {}})
    seen = "no button"
    for _ in range(60):
        seen = chrome(m, """
          const b = document.getElementById('formicaria-settings');
          if (!b) return 'no button';
          const r = b.getBoundingClientRect();
          return r.width > 0 && r.height > 0 ? 'drawn' : 'not drawn';""")
        if seen == "drawn":
            break
        time.sleep(0.5)
    took = time.time() - t0
    st = bc.post("/api/browser_status")
    check(who, "this release's kit is in place at once", st.get("kit_current") is True, json.dumps(st)[:300])
    check(who, "Settings is in the tab row at the first start, within 10 s", seen == "drawn" and took < 10,
          f"{seen} after {took:.1f} s")
    if seen == "drawn":
        pressed = click_in_window(m, "#formicaria-settings")
        time.sleep(3)
        shown = chrome(m, "return window.gBrowser.selectedBrowser.currentURI.spec")
        check(who, "and it opens Settings", pressed == "pressed" and shown.startswith(f"{SERVER}/f/settings"),
              f"{pressed}; shown {shown}")
    time.sleep(5)  # the background preparation, had it anything to do, has started by now
    st = bc.post("/api/browser_status")
    check(who, "nothing is left waiting for another restart", not st.get("update_ready"), json.dumps(st)[:300])
    try:
        m.cmd("Marionette:Quit", {"flags": ["eAttemptQuit"]})
    except Exception:  # noqa: BLE001 - the connection goes with the browser
        pass
    for _ in range(60):
        if not bc.procs_under(ff):
            break
        time.sleep(0.5)
    bc.stop(ff)


# **Real pages, kept through the app** (the owner, 2026-11-17: *"test with known pages with different
# contents ranging from images to video and sound: Wikipedia is perfect"*; *"check inside the app, not
# only stand alone"*). Free-licensed, stable addresses. Each: the page, the candidate to pick (a part of
# its label), which of those (a page with several: not always the first), how many it must list at
# least, what to keep, where it goes.
WIKI_MEDIA = [
    ("Ode to Joy: its recording, as MP3", "https://en.wikipedia.org/wiki/Ode_to_Joy", "Audio:", 0, 1, "mp3", "Music"),
    # **Many in one page** (the owner: *"include instances where there are many contents in a single
    # page"*): four recordings; all listed, the second kept (a short one: the third is a 141 MB FLAC, whose download time is the network's), and it is the second that is kept.
    ("Toccata and Fugue: the second of its recordings, as served",
     "https://en.wikipedia.org/wiki/Toccata_and_Fugue_in_D_minor,_BWV_565", "Audio:", 1, 4, "audio", "Music"),
    ("Ant Bed With Ants (Commons): the video at 240p",
     "https://commons.wikimedia.org/wiki/File:Ant_Bed_With_Ants.webm", "Video:", 0, 1, "video", "Videos"),
]
WIKI_PAGE = ("Ant: the article and its pictures", "https://en.wikipedia.org/wiki/Ant")
TOOLS_CACHE = bc.CACHE / "media-tools"


def handles(m) -> list:
    h = m.cmd("WebDriver:GetWindowHandles")
    return h["value"] if isinstance(h, dict) else h


def new_tab(m, url: str, settle: float = 6) -> str:
    """A new tab showing `url`, made from a tab that still exists: the kit closes the tabs a privacy
    extension opens on install, and the session may have been standing in one of them."""
    m.cmd("WebDriver:SwitchToWindow", {"handle": handles(m)[0]})
    r = m.cmd("WebDriver:NewWindow", {"type": "tab"})
    h = (r.get("value") or r)["handle"] if isinstance(r, dict) else handles(m)[-1]
    m.cmd("WebDriver:SwitchToWindow", {"handle": h})
    goto(m, url, settle)
    return h


def handles_where(m, part: str) -> str | None:
    for h in handles(m):
        m.cmd("WebDriver:SwitchToWindow", {"handle": h})
        if part in m.cmd("WebDriver:GetCurrentURL")["value"]:
            return h
    return None


def close_tabs(m, *parts: str) -> None:
    """Closes the tabs showing any of `parts` (a case's page and its sheet), keeping one tab open."""
    for h in handles(m):
        m.cmd("WebDriver:SwitchToWindow", {"handle": h})
        if any(p in m.cmd("WebDriver:GetCurrentURL")["value"] for p in parts) and len(handles(m)) > 1:
            m.cmd("WebDriver:CloseWindow")
    m.cmd("WebDriver:SwitchToWindow", {"handle": handles(m)[0]})


def ask_addon(m, host: str, page_part: str, kind: str, wait_ms: int = 60000):
    """The ant's panel, opened as a page, asks the add-on to act on the tab showing `page_part`; what
    the add-on answered. In a tab of its own: never over the page it is about."""
    new_tab(m, f"moz-extension://{host}/launcher.html", 2)
    m.cmd("WebDriver:SetTimeouts", {"script": wait_ms})
    r = m.cmd("WebDriver:ExecuteAsyncScript", {"args": [page_part, kind], "scriptTimeout": wait_ms, "script": """
        const [part, kind, done] = arguments;
        const b = window.browser || (window.wrappedJSObject || {}).browser;
        b.tabs.query({}).then((ts) => {
          const t = ts.find((x) => (x.url || '').includes(part));
          if (!t) return done('no tab');
          b.runtime.sendMessage({ type: kind, tab: t.id }).then((r) => done(JSON.stringify({ r: r ?? null, tab: t.url, status: t.status })), (e) => done(JSON.stringify({ r: String(e), tab: t.url })));
        });"""})
    v = r.get("value") if isinstance(r, dict) else r
    try:
        out = json.loads(v)
    except (ValueError, TypeError):
        return v
    if isinstance(out, dict) and "tab" in out:
        say_tab = out.get("tab")
        out = out.get("r")
        if isinstance(out, dict):
            out["_tab"] = say_tab
        else:
            out = {"ok": False, "error": str(out), "_tab": say_tab}
    return out


def media_saver(ff: Path, work: Path) -> None:
    """Pages of Wikipedia and Commons, their sound and video kept through formicaria's browser: the
    add-on reads the live page, the Save media sheet is clicked, the server's tools fetch and file."""
    who = "someone keeping media from Wikipedia"
    m = launch(ff, work / "profile-media", f"{SERVER}/f/group/Library")
    try:
        time.sleep(5)
        host = chrome(m, f"return WebExtensionPolicy.getByID('{ADDON}')?.mozExtensionHostname")
        cases = [c for c in WIKI_MEDIA if os.environ.get("FM_WIKI_CASES", "all") == "all"
                 or any(k in c[0] for k in os.environ["FM_WIKI_CASES"].split(","))]
        for what_case, page, pick, nth, least, keep, into in cases:
            close_tabs(m, "/f/media", "wikipedia.org", "wikimedia.org", "moz-extension:")
            new_tab(m, page)
            ask_addon(m, host, page.split("/wiki/")[1][:20], "save-media")
            time.sleep(3)
            if not handles_where(m, "/f/media"):
                check(who, f"{what_case}: the Save media sheet opens", False, "no /f/media tab")
                continue
            listed = wait_for(m, f"return [...document.querySelectorAll('input[name=candidate]')].map(i=>i.parentElement.textContent.trim())", 20) or []
            matching = [x for x in listed if pick in x]
            check(who, f"{what_case}: the sheet lists all {least}+ of them", len(matching) >= least, f"listed: {listed}")
            chosen = content(m, f"""const l=[...document.querySelectorAll('input[name=candidate]')].filter(i=>i.parentElement.textContent.includes({json.dumps(pick)}))[{nth}];
                                   if(l) l.click(); return l ? l.parentElement.textContent.trim() : null;""")
            check(who, f"{what_case}: the one wanted can be chosen", bool(chosen), f"listed: {listed}")
            if not chosen:
                continue
            # Looked at (the tools fetched the first time: allow a few minutes).
            seen = wait_for(m, "return !!document.querySelector('input[name=what]') || (document.querySelector('[role=alert]')||{}).textContent", 600)
            if seen is not True:
                check(who, f"{what_case}: it is looked at", False, str(seen))
                continue
            label = {"mp3": "The sound as MP3", "audio": "The sound, as the site serves it", "video": "The video"}[keep]
            content(m, f"""const r=[...document.querySelectorAll('input[name=what]')].find(i=>i.parentElement.textContent.includes({json.dumps(label)})); if(r) r.click();""")
            time.sleep(1)
            if keep == "video":
                content(m, "const s=document.querySelector('select[aria-label=\"video quality\"]'); if(s){const o=[...s.options].map(o=>+o.value).filter(v=>v>=240).sort((a,b)=>a-b)[0]; if(o){s.value=String(o); s.dispatchEvent(new Event('change',{bubbles:true}));}}")
            content(m, "[...document.querySelectorAll('button')].find(b=>b.textContent.trim()==='Save').click()")
            done = wait_for(m, f"const t=document.body.innerText; return t.includes('Saved in {into}') ? 'ok' : (t.match(/Not saved[^\\n]*/)||[null])[0]", 600)
            said = "" if done == "ok" else content(m, "return document.body.innerText.slice(-400)")
            check(who, f"{what_case}: the sheet says it is saved in {into}", done == "ok", f"{done}; the sheet: {said!r}")
            lib = bc.post("/api/music_library" if into == "Music" else "/api/video_library")
            mine = [n for n in lib if page.split("/wiki/")[1].split(":")[-1][:12] in json.dumps(n)]
            check(who, f"{what_case}: its note is in {into}, with where it came from", bool(mine), f"{len(lib)} notes")
            # The one chosen, not another of the page's: its file's name is in the note's source.
            name = chosen.split(":", 1)[-1].strip()[:20]
            check(who, f"{what_case}: it is the one chosen", any(name.split(".")[0][:12] in json.dumps(n) for n in mine),
                  f"chose {chosen!r}; notes {[n.get('props', {}).get('source') for n in mine]}")
            if mine and keep == "mp3":
                ref = mine[0]["props"].get("audio", "")
                with urllib.request.urlopen(f"{SERVER}/api/blob/{ref}", timeout=30) as r:
                    head = r.read(3)
                check(who, f"{what_case}: the file kept is an MP3", head in (b"ID3",) or head[:1] == b"\xff", repr(head))
        # **A saved song played, as a person plays it** (the owner, 2026-11-20: *"the music view is not
        # playing when I click play … test both anyway"*): Music, *play* pressed the way a mouse
        # presses it (a script's click carries no gesture, which autoplay rules may refuse), then what
        # the player bar shows eight seconds later.
        if os.environ.get("FM_WIKI_CASES", "all") in ("all", "play"):
            close_tabs(m, "/f/media", "wikipedia.org", "wikimedia.org", "moz-extension:")
            new_tab(m, f"{SERVER}/f/group/Music")
            song = wait_for(m, "const b=[...document.querySelectorAll('button')].find(b=>(b.getAttribute('aria-label')||'').startsWith('play ')); return b ? b.getAttribute('aria-label') : null", 30)
            check(who, "a song saved here is in Music with its play button", bool(song),
                  str(content(m, "return document.body.innerText.slice(0, 300)")))
            if song:
                try:
                    el = m.cmd("WebDriver:FindElement", {"using": "css selector",
                                                         "value": 'button[aria-label^="play "]'})["value"]
                    m.cmd("WebDriver:ElementClick", {"id": next(iter(el.values()))})
                    pressed = True
                except RuntimeError as e:
                    pressed = False
                    check(who, "its play button can be pressed", False, str(e)[:300])
                if pressed:
                    time.sleep(8)
                    st = content(m, """const p=document.querySelector('input[aria-label="position in the song"]');
                        return {pos: p ? +p.value : null, pausing: !!document.querySelector('button[aria-label=pause]'),
                                said: (document.body.innerText.match(/cannot be played[^\\n]*|could not[^\\n]*/)||[''])[0]}""")
                    check(who, "it plays when play is pressed: the player offers pause, says no error",
                          bool(st.get("pausing")) and not st.get("said"), str(st))
                    check(who, "its position moves on while it plays", (st.get("pos") or 0) >= 2, str(st))
                    content(m, "document.querySelector('button[aria-label=pause]')?.click()")
        # **A selection, brought in the way that fits it** (the ant's chooser by a selection): the
        # page's own markup for a formula and a recording, as the selection carries it.
        if os.environ.get("FM_WIKI_CASES", "all") in ("all", "selection"):
            for what_case, page, css, how, want in [
                ("a formula on Pythagorean theorem, as LaTeX", "https://en.wikipedia.org/wiki/Pythagorean_theorem",
                 ".mw-parser-output .mwe-math-element", "latex", "formula"),
                ("Ode to Joy's recording, selected, as its sound", "https://en.wikipedia.org/wiki/Ode_to_Joy",
                 ".mw-parser-output audio", "mp3", "sound"),
            ]:
                close_tabs(m, "/f/media", "wikimedia.org", "moz-extension:", "wikipedia.org")
                new_tab(m, page)
                html = content(m, f"const e=document.querySelector({json.dumps(css)}); return e ? e.outerHTML : null")
                if not html:
                    check(who, f"{what_case}: the page has it", False, f"no {css}")
                    continue
                new_tab(m, f"moz-extension://{host}/launcher.html", 2)
                m.cmd("WebDriver:SetTimeouts", {"script": 60000})
                r = m.cmd("WebDriver:ExecuteAsyncScript", {"args": [html, how], "script": """
                    const [html, how, done] = arguments;
                    const b = window.browser || (window.wrappedJSObject || {}).browser;
                    b.runtime.sendMessage({ type: 'bring-selection', html, text: '', fresh: true, how })
                      .then((r) => done(JSON.stringify(r)), (e) => done(JSON.stringify({ ok: false, error: String(e) })));"""})
                got = json.loads(r["value"] if isinstance(r, dict) else r)
                check(who, f"{what_case}: brought in", bool(got.get("ok")), str(got)[:300])
                if want == "formula" and got.get("note"):
                    body = bc.post("/api/get", {"id": got["note"]}).get("body", "")
                    import re
                    tex = re.search(r"\$[^$\n]+\$", body)
                    check(who, f"{what_case}: the note holds it as LaTeX", bool(tex) and "displaystyle" not in body,
                          body[:200])
                if want == "sound":
                    check(who, f"{what_case}: it is being fetched into Music", "Music" in str(got.get("say", "")),
                          str(got)[:300])
                    found = None
                    for _ in range(120):
                        found = [n for n in bc.post("/api/music_library") if "Anthem" in json.dumps(n)]
                        if len(found) >= 2:  # one from the Save media case above, one from here
                            break
                        time.sleep(3)
                    check(who, f"{what_case}: a song note arrives in Music", bool(found) and len(found) >= (2 if os.environ.get("FM_WIKI_CASES", "all") == "all" else 1),
                          f"{len(found or [])} Anthem notes")

        # **Snip in a PDF** (Firefox's viewer admits no add-on script): the tab captured, the area
        # dragged on the add-on's own page, brought in as a picture.
        if os.environ.get("FM_WIKI_CASES", "all") in ("all", "selection", "pdf"):
            pdf = "https://www.w3.org/WAI/ER/tests/xhtml/testfiles/resources/pdf/dummy.pdf"
            close_tabs(m, "/f/media", "wikimedia.org", "moz-extension:", "wikipedia.org")
            new_tab(m, pdf, 8)
            new_tab(m, f"moz-extension://{host}/launcher.html", 2)
            m.cmd("WebDriver:SetTimeouts", {"script": 30000})
            m.cmd("WebDriver:ExecuteAsyncScript", {"args": [pdf], "script": """
                const [pdf, done] = arguments;
                const b = window.browser || (window.wrappedJSObject || {}).browser;
                b.tabs.query({}).then((ts) => {
                  const t = ts.find((x) => x.url === pdf);
                  if (!t) return done('no tab');
                  b.runtime.sendMessage({ type: 'snip-tab', tab: t.id }).then(() => done('ok'), (e) => done(String(e)));
                });"""})
            time.sleep(4)
            shot = handles_where(m, "/shot.html")
            check(who, "a PDF: Snip opens its page", bool(shot), "no snip page")
            if shot:
                drawn = wait_for(m, "const i=document.getElementById('shot'); return i && i.naturalWidth > 0 && i.naturalWidth", 20)
                check(who, "a PDF: the snip page shows what the tab showed", bool(drawn), str(drawn))
                content(m, """
                    const st = document.getElementById('stage'), r = document.getElementById('shot').getBoundingClientRect();
                    const ev = (t, x, y) => st.dispatchEvent(new PointerEvent(t, { bubbles: true, clientX: r.left + x, clientY: r.top + y, pointerId: 1 }));
                    ev('pointerdown', 20, 20); ev('pointermove', 260, 140); ev('pointerup', 260, 140);
                    document.getElementById('bring').click();""")
                said = wait_for(m, "const t=document.getElementById('say')?.textContent||''; return /^(Into|Not brought)/.test(t) && t", 30)
                check(who, "a PDF: the area is brought into a note", str(said or "").startswith("Into"), str(said))

        # The article, kept whole: its pictures inside the copy.
        name, page = WIKI_PAGE
        page = os.environ.get("FM_WIKI_PAGE", page)
        close_tabs(m, "/f/media", "wikimedia.org", "moz-extension:")
        new_tab(m, page)
        # Its pictures are fetched and put inside the copy: allow two minutes, and say how long.
        m.s.settimeout(200)
        t0 = time.time()
        kept = ask_addon(m, host, page.split(".org")[1], "save-page", 180000)
        took = time.time() - t0
        m.s.settimeout(30)
        print(f"usability: [{who}] {name}: saved in {took:.0f} s", flush=True)
        check(who, f"{name}: saved within two minutes", took < 120, f"{took:.0f} s")
        note = bc.post("/api/get", {"id": kept["id"]}) if isinstance(kept, dict) and kept.get("ok") else None
        check(who, f"{name}: kept as a note", bool(note), f"the add-on said {kept!r}"[:300])
        if note:
            body = note.get("body", "")
            refs = [l.split("asset:")[1].split(")")[0] for l in body.splitlines() if "asset:" in l]
            copy = b""
            if refs:
                with urllib.request.urlopen(f"{SERVER}/api/blob/{refs[-1].replace('sha256-', 'sha256:')}", timeout=60) as r:
                    copy = r.read()
            pictures = copy.count(b"data:image/")
            check(who, f"{name}: its copy holds its pictures", pictures >= 3, f"{pictures} pictures inside, {len(copy)} bytes")
    finally:
        m.cmd("WebDriver:DeleteSession")
        bc.stop(ff)


def saved_page_reader(work: Path) -> None:
    who = "someone reading a saved page"
    html = b"<!doctype html><html><head><title>Ants</title></head><body><h1>Ants march on</h1><img src='data:image/png;base64,AAAA'></body></html>"
    req = urllib.request.Request(f"{SERVER}/api/ingest?name=ants.html&vault=", data=html, method="POST",
                                 headers={"Content-Type": "application/octet-stream"})
    asset = json.load(urllib.request.urlopen(req, timeout=30))
    ref = asset["assets"][0]
    url = f"{SERVER}/api/blob/{ref}?kind=view"
    with urllib.request.urlopen(url, timeout=30) as r:
        h = {k.lower(): v for k, v in r.headers.items()}
        body = r.read()
    check(who, "the copy is served as a page", h.get("content-type", "").startswith("text/html"), f"{h.get('content-type')}")
    check(who, "it is not a download", "content-disposition" not in h, f"{h.get('content-disposition')}")
    csp = h.get("content-security-policy", "")
    check(who, "nothing in it can run", "sandbox" in csp and "allow-scripts" not in csp and "script-src" not in csp, csp)
    check(who, "its pictures are allowed", "img-src data:" in csp, csp)
    check(who, "the bytes are the page's", body == html, f"{len(body)} bytes")


def main() -> None:
    work = bc.CACHE / "usability"
    bc.run(["pnpm", "-C", "ui", "build"], stdout=subprocess.DEVNULL)
    bc.run(["cargo", "build", "-q", "-p", "fm-serve"])
    if work.exists():
        bc.stop(work / "config" / "formicaria" / "browser" / "firefox")
        shutil.rmtree(work)
    (work / "config").mkdir(parents=True)
    (work / "vault").mkdir()
    env = dict(os.environ, XDG_CONFIG_HOME=str(work / "config"), FM_UI_DIST="ui/dist",
               FM_VAULT=str(work / "vault"), FM_VAULTS=str(work / "vaults.json"), FM_OPEN="0",
               FM_AUTO_SHUTDOWN="0", FM_ADDR=f"127.0.0.1:{bc.PORT}",
               # The usage diary's sample and counts every 15 s, not 5 min, so this run sees them.
               FM_DIARY_EVERY="15",
               # The browser the server opens can be driven (and drawn nowhere), for the second day.
               MOZ_MARIONETTE="1", MOZ_REMOTE_ALLOW_SYSTEM_ACCESS="1", MOZ_HEADLESS="1")
    # Its words kept (2026-11-21: it stopped once while setting its browser up, and nothing said why).
    server_log = open(work / "server.log", "w")
    server = subprocess.Popen([str(ROOT / "target" / "debug" / "fm-serve")], cwd=ROOT, env=env,
                              stdout=server_log, stderr=subprocess.STDOUT)
    try:
        bc.wait_until_up()
        bc.post("/api/set_identity", {"name": "Check Reader", "email": "reader@example.org"})
        bc.post("/api/browser_setup")
        s = {}
        for _ in range(600):
            s = bc.post("/api/browser_status")
            if not s.get("busy"):
                break
            time.sleep(1)
        if s.get("error") or not s.get("installed"):
            bc.fail(f"setup: {s.get('error')} (the whole status: {json.dumps(s)[:400]})")
        ff = work / "config" / "formicaria" / "browser" / "firefox"
        # The media tools, kept between runs (yt-dlp and ffmpeg, ~150 MB): checked against the pins
        # by the server as on any computer; only not downloaded each time.
        tools = work / "config" / "formicaria" / "tools" / "media"
        if TOOLS_CACHE.exists():
            shutil.copytree(TOOLS_CACHE, tools, dirs_exist_ok=True)
        personas = {
            "returning": lambda: returning_user(ff, work), "new": lambda: new_person(ff, work),
            "second-day": lambda: opening_on_a_second_day(ff),
            "left-on-settings": lambda: left_on_settings(ff),
            "update-bar": lambda: update_bar(ff),
            "updated": lambda: after_an_update(ff),
            "privacy": lambda: privacy_minded(ff, work),
            "settings": lambda: settings_alone(ff, work),
            "storage": lambda: storage_cleanup(ff, work),
            "music": lambda: music_player(ff, work),
            "width": lambda: reading_width(ff, work),
            "look": lambda: look(ff, work),
            "audit": lambda: (first_hour_pictures(ff, work), first_hour_walk(ff, work)),
            "basics": lambda: first_hour_basics(ff, work),
            "reader": lambda: saved_page_reader(work),
            "media": lambda: media_saver(ff, work),
        }
        # `FM_USABILITY_ONLY=media` runs one while working on it; a release runs them all.
        only = os.environ.get("FM_USABILITY_ONLY", "").split(",")
        for name, persona in personas.items():
            if only != [""] and name not in only:
                continue
            if only == [""] and name in ("look", "audit"):
                continue  # pictures, on request
            try:
                persona()
            except Exception as e:  # noqa: BLE001 - a persona that cannot finish is a failure, said
                where = [f"{f.lineno}" for f in traceback.extract_tb(e.__traceback__) if f.filename == __file__]
                check("harness", "a persona ran to the end", False, f"{repr(e)[:300]} (lines {', '.join(where)})")
                bc.stop(ff)
        diary_kept(work)
    finally:
        server.terminate()
        if (work / "config" / "formicaria" / "tools" / "media").exists():
            shutil.copytree(work / "config" / "formicaria" / "tools" / "media", TOOLS_CACHE, dirs_exist_ok=True)
    failed = [r for r in results if not r[2]]
    print(f"usability: {len(results) - len(failed)} of {len(results)} checks passed")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
