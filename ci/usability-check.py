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
        for want in ["Block trackers", "Block ads", "Refuse cookie pop-ups", "Clean tracking links", "More sites, with extensions", "DuckDuckGo's protections too"]:
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
               # The browser the server opens can be driven (and drawn nowhere), for the second day.
               MOZ_MARIONETTE="1", MOZ_REMOTE_ALLOW_SYSTEM_ACCESS="1", MOZ_HEADLESS="1")
    server = subprocess.Popen([str(ROOT / "target" / "debug" / "fm-serve")], cwd=ROOT, env=env,
                              stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        time.sleep(2)
        bc.post("/api/set_identity", {"name": "Check Reader", "email": "reader@example.org"})
        bc.post("/api/browser_setup")
        s = {}
        for _ in range(600):
            s = bc.post("/api/browser_status")
            if not s.get("busy"):
                break
            time.sleep(1)
        if s.get("error") or not s.get("installed"):
            bc.fail(f"setup: {s.get('error')}")
        ff = work / "config" / "formicaria" / "browser" / "firefox"
        # The media tools, kept between runs (yt-dlp and ffmpeg, ~150 MB): checked against the pins
        # by the server as on any computer; only not downloaded each time.
        tools = work / "config" / "formicaria" / "tools" / "media"
        if TOOLS_CACHE.exists():
            shutil.copytree(TOOLS_CACHE, tools, dirs_exist_ok=True)
        personas = {
            "returning": lambda: returning_user(ff, work), "new": lambda: new_person(ff, work),
            "second-day": lambda: opening_on_a_second_day(ff), "reader": lambda: saved_page_reader(work),
            "media": lambda: media_saver(ff, work),
        }
        # `FM_USABILITY_ONLY=media` runs one while working on it; a release runs them all.
        only = os.environ.get("FM_USABILITY_ONLY", "").split(",")
        for name, persona in personas.items():
            if only != [""] and name not in only:
                continue
            try:
                persona()
            except Exception as e:  # noqa: BLE001 - a persona that cannot finish is a failure, said
                where = [f"{f.lineno}" for f in traceback.extract_tb(e.__traceback__) if f.filename == __file__]
                check("harness", "a persona ran to the end", False, f"{repr(e)[:300]} (lines {', '.join(where)})")
                bc.stop(ff)
    finally:
        server.terminate()
        if (work / "config" / "formicaria" / "tools" / "media").exists():
            shutil.copytree(work / "config" / "formicaria" / "tools" / "media", TOOLS_CACHE, dirs_exist_ok=True)
    failed = [r for r in results if not r[2]]
    print(f"usability: {len(results) - len(failed)} of {len(results)} checks passed")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
