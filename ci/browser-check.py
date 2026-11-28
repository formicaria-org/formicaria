#!/usr/bin/env python3
"""The release gate for formicaria's browser (`browser-plan` §13.1, §16; `decisions.md` 2026-11-15, evening).

On this computer, isolated (its own settings folder, vault, port and profiles; the person's own Firefox
and formicaria browser are never touched):

1. builds the UI and `fm-serve`, starts it, and sets formicaria's browser up through the app's own route
   (the pinned ESR from Mozilla, signature-checked, the kit applied, the self-check);
2. starts it headless on formicaria and three real pages, and checks what a person would see: the name,
   the pinned formicaria tab, the formicaria button, the policies (Mozilla's services off, Sync kept), the
   one locked line;
3. measures memory (PSS, every process) and start against **the same ESR, unmodified**, and fails the
   release past the budgets: memory > +10 %, start > +15 % (+100 ms of noise).
"""
import base64
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import tarfile
import time
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CACHE = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "formicaria-browser-check"
PORT = 8797
SERVER = f"http://127.0.0.1:{PORT}"
PAGES = [
    "https://en.wikipedia.org/wiki/Ant",
    "https://arxiv.org/abs/1706.03762",
    "https://ricette.giallozafferano.it/Spaghetti-alla-Carbonara.html",
]
MEMORY_BUDGET = 1.10
START_BUDGET = 1.15
START_NOISE_MS = 100
# Three again (2026-11-18, `lean-plan` L8): five were for a first start whose memory moved by 200 MB
# between identical runs (2026-11-16); measured warm, the starts sit within 10 MB of each other.
RUNS = 3

MEASURE_CFG = """// measurement only (no kit)
try {
  const { setTimeout } = ChromeUtils.importESModule("resource://gre/modules/Timer.sys.mjs");
  const path = Services.env.get("FM_BROWSER_CHECK");
  Services.obs.addObserver(function seen(win) {
    Services.obs.removeObserver(seen, "browser-delayed-startup-finished");
    setTimeout(() => {
      const i = Services.startup.getStartupInfo();
      const r = { sessionRestored_ms: i.sessionRestored ? i.sessionRestored.getTime() - i.process.getTime() : null };
      const f = Cc["@mozilla.org/file/local;1"].createInstance(Ci.nsIFile); f.initWithPath(path);
      const s = Cc["@mozilla.org/network/file-output-stream;1"].createInstance(Ci.nsIFileOutputStream);
      s.init(f, 0x02 | 0x08 | 0x20, 0o644, 0); const t = JSON.stringify(r); s.write(t, t.length); s.close();
    }, 8000);
  }, "browser-delayed-startup-finished");
} catch (e) {}
"""


def fail(msg: str) -> None:
    print(f"browser-check: FAIL — {msg}")
    sys.exit(1)


def run(cmd: list[str], **kw) -> None:
    print("browser-check: $", " ".join(cmd))
    subprocess.run(cmd, check=True, cwd=ROOT, **kw)


def post(path: str, body: dict | None = None) -> dict:
    req = urllib.request.Request(SERVER + path, data=json.dumps(body or {}).encode(), method="POST")
    with urllib.request.urlopen(req, timeout=30) as r:
        return json.loads(r.read() or b"{}")


def wait_until_up(secs: int = 60) -> None:
    """The test server answers (`/api/alive`), not a guessed pause: on a busy machine it took longer
    than the two seconds once waited (2026-11-19)."""
    for _ in range(secs * 4):
        try:
            urllib.request.urlopen(f"{SERVER}/api/alive", timeout=2).read()
            return
        except Exception:  # noqa: BLE001 - not up yet
            time.sleep(0.25)
    fail("the test server did not start")


def procs_under(folder: Path) -> list[int]:
    out = []
    for pid in os.listdir("/proc"):
        if not pid.isdigit():
            continue
        try:
            exe = os.readlink(f"/proc/{pid}/exe")
        except OSError:
            continue
        if exe.startswith(str(folder) + "/"):
            out.append(int(pid))
    return out


def pss_kb(folder: Path) -> int:
    total = 0
    for pid in procs_under(folder):
        try:
            for line in Path(f"/proc/{pid}/smaps_rollup").read_text().splitlines():
                if line.startswith("Pss:"):
                    total += int(line.split()[1])
        except OSError:
            pass
    return total


def stop(folder: Path) -> None:
    for pid in procs_under(folder):
        try:
            os.kill(pid, signal.SIGTERM)
        except OSError:
            pass
    time.sleep(3)


def measure(firefox: Path, profile: Path, report: Path, urls: list[str], extra_env: dict,
            warm: bool = False) -> dict:
    """Start `firefox` headless on `urls`, wait, read its report and its memory, stop it.

    `warm`: on a profile that has already started once, as a person's browser is every day after the
    first (2026-11-18). At a first start formicaria's privacy extensions are still downloading and
    installing, and how far they had got moved its memory by up to 300 MB between identical runs; the
    first start's work (the extensions, the privacy switches) is then settled, not raced."""
    if profile.exists():
        shutil.rmtree(profile)
    profile.mkdir(parents=True)
    report.unlink(missing_ok=True)
    env = dict(os.environ, FM_BROWSER_CHECK=str(report), **extra_env)
    if warm:
        subprocess.Popen(
            [str(firefox / "firefox"), "--headless", "--no-remote", "--name", "formicaria-check",
             "--profile", str(profile), *urls],
            env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        )
        time.sleep(45)
        stop(firefox)
        for _ in range(30):  # gone, so the profile is free for the second start
            if not procs_under(firefox):
                break
            time.sleep(1)
        # The same pages again, not the warm-up's tabs restored beside them (formicaria restores the
        # last session; unmodified Firefox does not), so both open exactly `urls`.
        (profile / "sessionstore.jsonlz4").unlink(missing_ok=True)
        shutil.rmtree(profile / "sessionstore-backups", ignore_errors=True)
        report.unlink(missing_ok=True)
    subprocess.Popen(
        [str(firefox / "firefox"), "--headless", "--no-remote", "--name", "formicaria-check",
         "--profile", str(profile), *urls],
        env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )
    time.sleep(30)
    kb = pss_kb(firefox)
    data = json.loads(report.read_text()) if report.exists() else {}
    stop(firefox)
    data["pss_kb"] = kb
    return data


class Marionette:
    """A minimal client for Firefox's remote protocol (length-prefixed JSON on port 2828)."""

    def __init__(self, port: int = 2828):
        import socket
        for _ in range(60):
            try:
                self.s = socket.create_connection(("127.0.0.1", port), timeout=30)
                break
            except OSError:
                time.sleep(1)
        else:
            fail("could not reach the browser's remote protocol")
        self.n = 0
        self._read()

    def _read(self):
        buf = b""
        while b":" not in buf:
            buf += self.s.recv(1)
        size, rest = buf.split(b":", 1)
        while len(rest) < int(size):
            rest += self.s.recv(int(size) - len(rest))
        return json.loads(rest)

    def cmd(self, name: str, params: dict | None = None):
        self.n += 1
        body = json.dumps([0, self.n, name, params or {}]).encode()
        self.s.sendall(str(len(body)).encode() + b":" + body)
        while True:
            r = self._read()
            if r[0] == 1 and r[1] == self.n:
                if r[2]:
                    raise RuntimeError(f"{name}: {r[2]}")
                return r[3]


TEST_PORT = 8796
TEST_PAGE = f"http://localhost:{TEST_PORT}/page.html"
# A 1x1 PNG, so the image test has real bytes to keep.
PIXEL = base64.b64decode(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg=="
)


def serve_test_page(work: Path) -> subprocess.Popen:
    """A page holding one of each kind, on `localhost` (formicaria's own `127.0.0.1` is not a web page)."""
    d = work / "testpage"
    d.mkdir(parents=True, exist_ok=True)
    (d / "pixel.png").write_bytes(PIXEL)
    (d / "page.html").write_text(
        "<!doctype html><html><head><title>Test walks</title></head><body>"
        "<p id=p>The path <b>climbs</b> gently through the pines.</p>"
        "<p><img id=img src=pixel.png alt='A pixel'></p>"
        "<table id=t><tr><th>Day</th><th>Km</th></tr><tr><td>Mon</td><td>12</td></tr></table>"
        "<p id=q>Ants carry leaves home along the trail.</p>"
        "</body></html>"
    )
    return subprocess.Popen([sys.executable, "-m", "http.server", str(TEST_PORT), "--bind", "127.0.0.1",
                             "--directory", str(d)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def bring_in(m: "Marionette") -> dict:
    """Each way in, through the add-on's own commands (the harness's synthetic keys do not reach add-on
    shortcuts; `doCommand` runs exactly what the key runs)."""
    out: dict = {}
    nid = post("/api/create_notes", {"notes": [{"title": "Reading", "body": "Start."}]})[0]["id"]
    m.cmd("Marionette:SetContext", {"value": "chrome"})
    # The note in a tab of its own, looked at (the add-on takes it as the note in front), then the page:
    # every piece goes into the note while it is not on screen (`put`).
    m.cmd("WebDriver:ExecuteScript", {"args": [SERVER + "/f/note/" + nid], "script": """
        const sp = Services.scriptSecurityManager.getSystemPrincipal();
        gBrowser.selectedTab = gBrowser.addTab(arguments[0], { triggeringPrincipal: sp });"""})
    time.sleep(5)
    m.cmd("WebDriver:ExecuteScript", {"args": [TEST_PAGE], "script": """
        const sp = Services.scriptSecurityManager.getSystemPrincipal();
        gBrowser.selectedTab = gBrowser.addTab(arguments[0], { triggeringPrincipal: sp });"""})
    time.sleep(5)
    handles = m.cmd("WebDriver:GetWindowHandles")
    handles = handles if isinstance(handles, list) else handles.get("value", handles)
    m.cmd("WebDriver:SwitchToWindow", {"handle": handles[-1]})

    def select(js: str) -> None:
        m.cmd("Marionette:SetContext", {"value": "content"})
        m.cmd("WebDriver:ExecuteScript", {"args": [], "script": f"""
            const r = document.createRange(); {js}
            getSelection().removeAllRanges(); getSelection().addRange(r);"""})

    def bring_key() -> str:
        m.cmd("Marionette:SetContext", {"value": "chrome"})
        m.cmd("WebDriver:ExecuteScript", {"args": [], "script": """
            [...document.querySelectorAll('key')].find(k => k.getAttribute('modifiers') === 'alt,shift' && k.getAttribute('key') === 'A').doCommand();"""})
        time.sleep(5)
        return post("/api/get", {"id": nid}).get("body", "")

    select("r.selectNodeContents(document.getElementById('p'));")
    body = bring_key()
    out["bring_text"] = "> The path **climbs** gently through the pines." in body and "> — [Test walks](" in body
    select("r.selectNode(document.getElementById('img'));")
    body = bring_key()
    out["bring_image"] = "![A pixel](asset:sha256-" in body
    select("r.selectNode(document.getElementById('t'));")
    body = bring_key()
    out["bring_table"] = "| Day | Km |" in body and "| Mon | 12 |" in body
    # The chip: a selection made with the pointer offers it, and pressing it brings the selection in.
    select("r.selectNodeContents(document.getElementById('q'));")
    m.cmd("WebDriver:ExecuteScript", {"args": [], "script": """
        document.getElementById('q').dispatchEvent(new MouseEvent('mouseup', { bubbles: true }));"""})
    time.sleep(1)
    has_chip = m.cmd("WebDriver:ExecuteScript", {"args": [], "script": """
        const c = document.querySelector('formicaria-chip'); if (c) c.click(); return !!c;"""})
    # The click opens the chooser with the note in front first, focused: Enter brings it there.
    time.sleep(0.5)
    m.cmd("WebDriver:PerformActions", {"actions": [{"type": "key", "id": "kb", "actions": [
        {"type": "keyDown", "value": "\ue007"}, {"type": "keyUp", "value": "\ue007"}]}]})
    m.cmd("WebDriver:ReleaseActions", {})
    time.sleep(5)
    body = post("/api/get", {"id": nid}).get("body", "")
    out["bring_chip"] = bool(has_chip.get("value") if isinstance(has_chip, dict) else has_chip) and "Ants carry leaves home" in body
    # **Snip**: Ctrl+Shift+drag from the first paragraph to the table brings all three in, each as itself.
    before = post("/api/get", {"id": nid}).get("body", "")
    m.cmd("WebDriver:ExecuteScript", {"args": [], "script": """
        getSelection().removeAllRanges();
        // Over the first paragraph's words, the image and the table, as a person would draw it.
        const words = (id) => { const r = document.createRange(); r.selectNodeContents(document.getElementById(id)); return r.getBoundingClientRect(); };
        const a = words('p');
        const b = document.getElementById('t').getBoundingClientRect();
        const right = Math.max(a.right, b.right) + 4;
        const ev = (type, x, y) => document.body.dispatchEvent(new MouseEvent(type, { bubbles: true, clientX: x, clientY: y, button: 0, ctrlKey: true, shiftKey: true }));
        ev('mousedown', a.left - 4, a.top - 4);
        ev('mousemove', right, b.bottom + 4);
        ev('mouseup', right, b.bottom + 4);"""})
    time.sleep(8)
    body = post("/api/get", {"id": nid}).get("body", "")
    new = body[len(before):] if body.startswith(before) else ""
    out["snip"] = ("> The path **climbs**" in new and "![A pixel](asset:sha256-" in new
                   and "| Day | Km |" in new and "Ants carry" not in new)
    if not out["snip"]:
        print(f"browser-check: the snip added {new!r}")
    if not all(out.values()):
        print(f"browser-check: the note read {body!r}")
    out["page_in_notes"] = any(n.get("id") == nid for n in post("/api/page_notes", {"url": TEST_PAGE}))
    return out


def star(m: "Marionette") -> dict:
    """**☆ makes a bookmark note too** (the owner, 2026-11-27: *"make ☆ save a bookmark note"*). The
    test page is open (`bring_in`); the star in the address bar is clicked as a mouse does. Firefox's
    bookmark is made, and a note with the page's title and link tagged `shelf:Bookmarks`; its panel
    says so. Pressed again, no second note. On another page, *Remove* in the panel takes both back."""
    out: dict = {}

    def bookmark_notes(url: str) -> list:
        named = post("/api/page_notes", {"url": url}) or []
        return [n for n in named if str(n.get("context", "")).startswith("source_url:")]

    def press_star() -> None:
        m.cmd("Marionette:SetContext", {"value": "chrome"})
        el = m.cmd("WebDriver:FindElement", {"using": "css selector", "value": "#star-button-box"})["value"]
        m.cmd("WebDriver:ElementClick", {"id": next(iter(el.values()))})

    def panel() -> dict:
        m.cmd("Marionette:SetContext", {"value": "chrome"})
        for _ in range(30):
            r = m.cmd("WebDriver:ExecuteScript", {"args": [], "script": """
                const p = document.getElementById('editBookmarkPanel');
                const line = document.getElementById('formicaria-bookmark-note');
                return { open: p?.state === 'open', said: line?.children[0]?.getAttribute('value') ?? '',
                         link: !!line && !line.children[1].hidden };"""})["value"]
            if r.get("open") and not r["said"].startswith("Saving"):
                return r
            time.sleep(0.5)
        return r

    def firefox_has(url: str) -> bool:
        m.cmd("Marionette:SetContext", {"value": "chrome"})
        return bool(m.cmd("WebDriver:ExecuteAsyncScript", {"args": [url], "scriptTimeout": 10000, "script": """
            const done = arguments[arguments.length - 1];
            PlacesUtils.bookmarks.fetch({ url: arguments[0] }).then((b) => done(!!b), () => done(false));"""})["value"])

    def hide() -> None:
        m.cmd("WebDriver:ExecuteScript", {"args": [], "script": "document.getElementById('editBookmarkPanel')?.hidePopup();"})
        time.sleep(1)

    press_star()
    first = panel()
    notes = bookmark_notes(TEST_PAGE)
    note = post("/api/get", {"id": notes[0]["id"]}) if notes else {}
    print(f"browser-check: ☆ panel {first}; bookmark notes {[(n.get('title'), n.get('context')) for n in notes]}")
    out["star_panel_says"] = first.get("said") == "Also saved in formicaria, as a note." and first.get("link") is True
    # Firefox saves a new bookmark when its panel closes.
    hide()
    kept = firefox_has(TEST_PAGE)
    out["star_note"] = (len(notes) == 1 and "shelf:Bookmarks" in (note.get("tags") or [])
                        and f"[Test walks]({TEST_PAGE})" in (note.get("body") or "") and kept)
    if not out["star_note"]:
        print(f"browser-check: ☆'s note {json.dumps(note)[:400]}; Firefox's bookmark {kept}")
    press_star()
    again = panel()
    out["star_again_no_copy"] = len(bookmark_notes(TEST_PAGE)) == 1 and again.get("said") == "Already in formicaria, as a note."
    hide()
    # Another page, then Remove in its panel: Firefox's bookmark and the note both go.
    other = TEST_PAGE + "?again"
    m.cmd("WebDriver:ExecuteScript", {"args": [other], "script": """
        const sp = Services.scriptSecurityManager.getSystemPrincipal();
        gBrowser.selectedTab = gBrowser.addTab(arguments[0], { triggeringPrincipal: sp });"""})
    time.sleep(4)
    press_star()
    made = panel()
    had = len(bookmark_notes(other))
    m.cmd("Marionette:SetContext", {"value": "chrome"})
    el = m.cmd("WebDriver:FindElement", {"using": "css selector", "value": "#editBookmarkPanelRemoveButton"})["value"]
    m.cmd("WebDriver:ElementClick", {"id": next(iter(el.values()))})
    time.sleep(3)
    out["star_remove_both"] = had == 1 and not bookmark_notes(other) and not firefox_has(other)
    print(f"browser-check: ☆ on another page: panel {made}, notes {had} then {len(bookmark_notes(other))}, "
          f"Firefox's bookmark kept {firefox_has(other)}")
    return out


def tabs(m: "Marionette") -> dict:
    """formicaria's features and notes as tabs (`decisions.md` 2026-11-17, *one server, many views*)."""
    nid = post("/api/create_notes", {"notes": [{"title": "Kept room", "body": "Here."}]})[0]["id"]
    m.cmd("Marionette:SetContext", {"value": "chrome"})
    r = m.cmd("WebDriver:ExecuteAsyncScript", {"args": [SERVER, nid], "scriptTimeout": 90000, "script": """
        const done = arguments[arguments.length - 1];
        const [server, id] = [arguments[0], arguments[1]];
        const sp = Services.scriptSecurityManager.getSystemPrincipal();
        const wait = (ms) => new Promise(r => setTimeout(r, ms));
        const gb = gBrowser;
        (async () => {
          const out = {};
          // A feature page stays a page, in a tab of its own, titled by what it shows.
          const t1 = gb.addTab(server + '/f/group/Tasks', { triggeringPrincipal: sp });
          const t2 = gb.addTab(server + '/f/note/' + id, { triggeringPrincipal: sp });
          gb.selectedTab = t1;
          await wait(6000);
          out.feature = { url: t1.linkedBrowser.currentURI.spec, title: t1.linkedBrowser.contentTitle, alive: !t1.closing && gb.tabs.includes(t1) };
          out.note = { url: t2.linkedBrowser.currentURI.spec, title: t2.linkedBrowser.contentTitle };
          // Two of formicaria's tabs side by side: Firefox's own split view.
          gb.addTabSplitView([t1, t2]);
          await wait(500);
          out.split = !!t1.splitview && !!t2.splitview;
          // The quick switch (Alt+Shift+X): back to the tab chosen before, whatever it holds.
          const p = gb.addTab('https://example.org/?q', { triggeringPrincipal: sp });
          await wait(1500);
          gb.selectedTab = t1; await wait(300);
          gb.selectedTab = p; await wait(300);
          out.swap = window.formicariaSwap();
          out.swapBack = gb.selectedTab === t1;
          // + suggests: formicaria's new-tab page (the features, a new note, the web), and on it the
          // tab row is the only row; a web page brings the address row back.
          const before = gb.tabs.length;
          BrowserCommands.openTab();
          await wait(3000);
          const navShown = () => document.getElementById('nav-bar').getBoundingClientRect().height > 0;
          out.plus = { more: gb.tabs.length === before + 1, url: gb.selectedBrowser.currentURI.spec,
                       marked: document.documentElement.hasAttribute('formicaria-page'),
                       navHeight: document.getElementById('nav-bar').getBoundingClientRect().height,
                       navCollapsed: document.getElementById('nav-bar').collapsed,
                       oneRow: document.documentElement.hasAttribute('formicaria-page') && !navShown() };
          gb.selectedTab = p; await wait(500);
          out.webRow = !document.documentElement.hasAttribute('formicaria-page') && navShown();
          gb.removeTab(gb.tabs[gb.tabs.length - 1]);
          // The old whole-app page in a tab becomes Notes.
          const whole = gb.addTab(server + '/', { triggeringPrincipal: sp });
          await wait(5000);
          out.wholeApp = whole.linkedBrowser.currentURI.spec;
          // A tab the panel years left becomes its item's page.
          const old = gb.addTab('about:blank', { triggeringPrincipal: sp });
          await wait(300);
          SessionStore.setCustomTabValue(old, 'formicaria-item', 'note:' + id);
          window.formicariaCarryOver(old);
          await wait(2000);
          out.carried = gb.tabs.filter(t => t.linkedBrowser.currentURI.spec === server + '/f/note/' + id).length;
          // Firefox kept bare: no home or account button on the toolbar (the tab row and address bar stay).
          const gone = id => { const b = document.getElementById(id); return !b || getComputedStyle(b).display === 'none'; };
          out.bare = gone('home-button') && gone('fxa-toolbar-menu-button') && !!document.getElementById('urlbar');
          done(out);
        })().catch(e => done({ error: String(e) }));"""})["value"]
    f, n, plus = r.get("feature") or {}, r.get("note") or {}, r.get("plus") or {}
    out = {
        "feature_tab": f.get("alive") is True and str(f.get("url", "")).endswith("/f/group/Tasks")
        and str(f.get("title", "")).startswith("Tasks"),
        "note_tab": str(n.get("url", "")).endswith("/f/note/" + nid) and str(n.get("title", "")).startswith("Kept room"),
        "split_formicaria_tabs": r.get("split") is True,
        "quick_switch": r.get("swap") == "tab" and r.get("swapBack") is True,
        "plus_suggests": plus.get("more") is True and str(plus.get("url", "")).endswith("/f/new"),
        "one_row": plus.get("oneRow") is True and r.get("webRow") is True,
        "whole_app_to_notes": str(r.get("wholeApp", "")).endswith("/f/home"),
        "old_tabs_carried": (r.get("carried") or 0) >= 2,
        "bare_toolbar": r.get("bare") is True,
    }
    if not all(out.values()):
        print(f"browser-check: tabs seen {r}")
    return out


def settings_page(ff: Path, work: Path) -> dict:
    """Open the real Settings page of formicaria's browser and say what a person would see there."""
    import base64
    profile = work / "settings-profile"
    if profile.exists():
        shutil.rmtree(profile)
    profile.mkdir(parents=True)
    # The panel test clicks a link from a script, which the pop-up blocker would stop.
    (profile / "user.js").write_text('user_pref("dom.disable_open_during_load", false);\n')
    subprocess.Popen(
        [str(ff / "firefox"), "--headless", "--marionette", "-remote-allow-system-access", "--no-remote",
         "--name", "formicaria-check", "--profile", str(profile), "about:blank"],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )
    try:
        m = Marionette()
        m.cmd("WebDriver:NewSession", {"capabilities": {}})
        m.cmd("WebDriver:Navigate", {"url": "about:preferences"})
        time.sleep(4)
        seen = m.cmd("WebDriver:ExecuteScript", {"args": [], "script": """
            const hidden = (sel) => { const e = document.querySelector(sel); return !e || getComputedStyle(e).display === 'none'; };
            const nav = Array.from(document.querySelectorAll('moz-page-nav-button')).map(b => b.textContent || b.getAttribute('label') || '');
            return {
              managed_notice_hidden: hidden('#policies-container-content'),
              new_look_promo_hidden: hidden('moz-promo.settings-redesign-promo'),
              ai_controls_hidden: hidden('#category-ai-features'),
              support_link_hidden: hidden('#helpButton'),
              about_formicaria: nav.some(t => /About formicaria/.test(t)),
            };"""})["value"]
        shot = m.cmd("WebDriver:TakeScreenshot", {"full": False})["value"]
        (CACHE / "settings.png").write_bytes(base64.b64decode(shot))
        m.cmd("Marionette:SetContext", {"value": "chrome"})
        mascot = m.cmd("WebDriver:ExecuteAsyncScript", {"args": [], "script": """
            const done = arguments[arguments.length - 1];
            fetch('chrome://global/skin/illustrations/kit-concerned.svg').then(r => r.text()).then(done, e => done('ERR ' + e));"""})["value"]
        ours = (ROOT / "browser" / "kit" / "branding" / "about-logo.svg").read_text()
        seen["mascot_is_formicarias"] = mascot.strip() == ours.strip()
        # A link a page opens in a new window becomes a tab of this window.
        links = m.cmd("WebDriver:ExecuteAsyncScript", {"args": [], "scriptTimeout": 30000, "script": """
            const done = arguments[arguments.length - 1];
            const gb = window.gBrowser;
            const sp = Services.scriptSecurityManager.getSystemPrincipal();
            const t = gb.addTab('data:text/html,<a id=a target=_blank href="https://example.org/?from=page">x</a><script>setTimeout(()=>document.getElementById("a").click(),300)</script>', { triggeringPrincipal: sp });
            gb.selectedTab = t;
            setTimeout(() => done({ linkTab: gb.tabs.some(x => x.linkedBrowser.currentURI.spec.includes('from=page')),
                                    windows: [...Services.wm.getEnumerator('navigator:browser')].length }), 4000);"""})["value"]
        seen["links_are_tabs"] = links.get("linkTab") is True and links.get("windows") == 1
        # **A sign-in popup holds only its page** (2026-11-25; the owner: a site's Google sign-in window
        # gained a tab of formicaria's Home). A page's `window.open` with features, as such sites do;
        # waited past the start-up rule's 4 s. The popup blocker is lifted in this copy for the step.
        popup = m.cmd("WebDriver:ExecuteAsyncScript", {"args": [], "scriptTimeout": 40000, "script": """
            const done = arguments[arguments.length - 1];
            const gb = window.gBrowser;
            const sp = Services.scriptSecurityManager.getSystemPrincipal();
            Services.prefs.setBoolPref('dom.disable_open_during_load', false);
            const before = new Set([...Services.wm.getEnumerator('navigator:browser')]);
            const t = gb.addTab('data:text/html,<script>setTimeout(()=>window.open("https://example.org/?signin=1","signin","width=480,height=600"),300)</script>', { triggeringPrincipal: sp });
            gb.selectedTab = t;
            setTimeout(() => {
              const wins = [...Services.wm.getEnumerator('navigator:browser')].filter(w => !before.has(w));
              const w = wins[0];
              const tabs = w ? [...w.gBrowser.tabs].map(x => x.linkedBrowser.currentURI.spec) : [];
              const hidden = w ? (w.document.documentElement.getAttribute('chromehidden') || '') : '';
              const popupLike = /\\b(toolbar|location)\\b/.test(hidden);
              for (const x of wins) x.close();
              gb.removeTab(t);
              Services.prefs.clearUserPref('dom.disable_open_during_load');
              done({ opened: wins.length, popupLike, hidden, tabs });
            }, 7000);"""})["value"]
        print(f"browser-check: a sign-in popup: {json.dumps(popup)[:300]}")
        seen["popup_is_its_page"] = (popup.get("opened") == 1 and popup.get("popupLike") is True
                                     and len(popup.get("tabs", [])) == 1
                                     and "signin=1" in popup["tabs"][0])
        # **The download question is its window alone** (2026-11-25; the owner: *"when I am downloading
        # something, it asks me where it should go in a window, but then opens a new tab with fm too"*).
        # A real download, as Firefox lists one: the add-on asks in a popup (`downloads.ts`); past the
        # start-up rule's 4 s, that window holds only the question.
        dl = m.cmd("WebDriver:ExecuteAsyncScript", {"args": [], "scriptTimeout": 40000, "script": """
            const done = arguments[arguments.length - 1];
            (async () => {
              const { Downloads } = ChromeUtils.importESModule('resource://gre/modules/Downloads.sys.mjs');
              const before = new Set([...Services.wm.getEnumerator('navigator:browser')]);
              const list = await Downloads.getList(Downloads.PUBLIC);
              const d = await Downloads.createDownload({ source: 'https://example.org/?download=1',
                target: PathUtils.join(PathUtils.tempDir, 'fm-download-check.html') });
              await list.add(d);
              d.start().catch(() => {});
              await new Promise((r) => setTimeout(r, 7000));
              const wins = [...Services.wm.getEnumerator('navigator:browser')].filter(w => !before.has(w));
              const tabs = wins.flatMap(w => [...w.gBrowser.tabs].map(x => x.linkedBrowser.currentURI.spec));
              const hidden = wins.map(w => w.document.documentElement.getAttribute('chromehidden') || '');
              const main = window.document.documentElement.getAttribute('chromehidden') || '';
              for (const w of wins) w.close();
              await d.finalize(true).catch(() => {});
              await list.remove(d).catch(() => {});
              return { opened: wins.length, tabs, hidden, main };
            })().then(done, (e) => done({ error: String(e) }));"""})["value"]
        print(f"browser-check: the download question: {json.dumps(dl)[:300]}")
        seen["download_question_alone"] = (dl.get("opened") == 1 and len(dl.get("tabs", [])) == 1
                                           and "choose.html" in dl["tabs"][0])
        # **Bring in**, end to end (research §11): a note in a tab of its own, then from a page each kind
        # of thing, and each must arrive in the note on disk as itself, with where it came from.
        try:
            seen.update(bring_in(m))
        except Exception as e:  # noqa: BLE001 - reported as failed checks
            print(f"browser-check: bring in: {e}")
        try:
            seen.update(star(m))
        except Exception as e:  # noqa: BLE001 - reported as failed checks
            print(f"browser-check: ☆: {e}")
        try:
            seen.update(tabs(m))
        except Exception as e:  # noqa: BLE001 - reported as failed checks
            print(f"browser-check: tabs: {e}")
        for k in ("bring_text", "bring_image", "bring_table", "bring_chip", "snip", "page_in_notes", "star_note", "star_panel_says", "star_again_no_copy", "star_remove_both", "links_are_tabs", "popup_is_its_page", "download_question_alone",
                  "feature_tab", "note_tab", "split_formicaria_tabs", "quick_switch", "plus_suggests", "one_row", "whole_app_to_notes",
                  "old_tabs_carried", "bare_toolbar"):
            seen.setdefault(k, False)
        m.cmd("WebDriver:DeleteSession")
        return seen
    finally:
        stop(ff)


# **Sites open as they do in Firefox** (2026-11-25; the owner: *"I have seen fm flagging a lot more
# websites than DuckDuckGo or Firefox … it is a symptom of a broader problem"*). Public pages of every
# sort a person meets: plain-http-only sites, ordinary https sites, a certificate Firefox repairs by
# itself (a missing intermediate), and two that every browser refuses (an expired certificate, a
# name the certificate does not cover) as controls. Each is opened in formicaria's browser and in the
# same ESR unmodified, on fresh profiles; what each shows is read as its kind of page. formicaria may
# never show a warning or an error page where Firefox shows the site.
SITES = [
    "http://neverssl.com/",
    "http://httpforever.com/",
    "http://http.badssl.com/",
    "http://info.cern.ch/",
    "http://example.com/",
    "https://scholar.google.com/",
    "https://en.wikipedia.org/wiki/Ant",
    "https://incomplete-chain.badssl.com/",
    "https://mixed-script.badssl.com/",
    "https://expired.badssl.com/",
    "https://www.scholar.google.com/",
]


def what_shows(firefox: Path, profile: Path, urls: list[str], user_js: str = "") -> dict:
    """Open each address in a fresh tab of `firefox` and say what it shows: `page`, or the error page's
    kind (`neterror:<code>`, `certerror`, `httpsonlyerror`, `blocked`)."""
    if profile.exists():
        shutil.rmtree(profile)
    profile.mkdir(parents=True)
    if user_js:
        (profile / "user.js").write_text(user_js)
    subprocess.Popen(
        [str(firefox / "firefox"), "--headless", "--marionette", "-remote-allow-system-access", "--no-remote",
         "--name", "formicaria-check", "--profile", str(profile), "about:blank"],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )
    out: dict = {}
    try:
        m = Marionette()
        m.cmd("WebDriver:NewSession", {"capabilities": {}})
        time.sleep(8)  # formicaria's start-up work (its switches, its extensions) settles first
        m.cmd("Marionette:SetContext", {"value": "chrome"})
        for url in urls:
            got = m.cmd("WebDriver:ExecuteAsyncScript", {"args": [url], "scriptTimeout": 40000, "script": """
                const [url, done] = [arguments[0], arguments[arguments.length - 1]];
                const gb = window.gBrowser;
                const t = gb.addTab(url, { triggeringPrincipal: Services.scriptSecurityManager.getSystemPrincipal() });
                gb.selectedTab = t;
                const read = () => {
                  const b = t.linkedBrowser;
                  const doc = b.documentURI ? b.documentURI.spec : '';
                  const m = /^about:(neterror|certerror|httpsonlyerror|blocked)(?:\\?e=([^&]*))?/.exec(doc);
                  return { shows: m ? (m[1] === 'neterror' ? `neterror:${m[2] || ''}` : m[1]) : 'page',
                           at: b.currentURI.spec };
                };
                // Long enough for a refused upgrade to fall back, or to give up.
                setTimeout(() => { const r = read(); gb.removeTab(t); done(r); }, 15000);"""})["value"]
            out[url] = got
    except Exception as e:  # noqa: BLE001 - said in the report
        out["error"] = repr(e)[:200]
    finally:
        stop(firefox)
        for _ in range(30):
            if not procs_under(firefox):
                break
            time.sleep(1)
    return out


def sites_as_firefox(ff: Path, base_ff: Path, work: Path) -> tuple[bool, str]:
    kit = what_shows(ff, work / "sites-kit", SITES)
    stock = what_shows(base_ff, work / "sites-stock", SITES)
    worse = []
    for url in SITES:
        k = (kit.get(url) or {}).get("shows", "none")
        b = (stock.get(url) or {}).get("shows", "none")
        print(f"browser-check: site  {url:<42} formicaria {k:<26} Firefox {b}")
        if k != "page" and b == "page":
            worse.append(f"{url} ({k})")
    for name, r in (("formicaria", kit), ("Firefox", stock)):
        if r.get("error"):
            worse.append(f"{name} could not be driven: {r['error']}")
    return not worse, "; ".join(worse)

def stock_esr(version: str, sha512: str) -> Path:
    """Mozilla's ESR, unmodified, with a measurement-only startup script: the baseline."""
    d = CACHE / f"stock-{version}"
    ff = d / "firefox"
    if not (ff / "firefox").is_file():
        d.mkdir(parents=True, exist_ok=True)
        tar = d / "ff.tar.xz"
        url = f"https://archive.mozilla.org/pub/firefox/releases/{version}/linux-x86_64/en-US/firefox-{version}.tar.xz"
        print(f"browser-check: downloading the unmodified baseline {version}")
        urllib.request.urlretrieve(url, tar)
        got = subprocess.run(["sha512sum", str(tar)], capture_output=True, text=True).stdout.split()[0]
        if got != sha512:
            fail("the baseline download does not match the pinned sum")
        with tarfile.open(tar) as t:
            t.extractall(d, filter="tar")
        tar.unlink()
    (ff / "defaults" / "pref" / "autoconfig.js").write_text(
        'pref("general.config.filename", "measure.cfg");\npref("general.config.obscure_value", 0);\n'
        'pref("general.config.sandbox_enabled", false);\n')
    (ff / "measure.cfg").write_text(MEASURE_CFG)
    return ff


def main() -> None:
    pin = (ROOT / "browser" / "esr.toml").read_text()
    version = re.search(r'^version = "([^"]+)"', pin, re.M).group(1)
    sha = re.search(r'^"linux-x86_64" = "([0-9a-f]{128})"', pin, re.M).group(1)
    run(["pnpm", "-C", "ui", "build"], stdout=subprocess.DEVNULL)
    run(["cargo", "build", "-q", "-p", "fm-serve"])

    work = CACHE / "run"
    if work.exists():
        stop(work / "config" / "formicaria" / "browser" / "firefox")
        shutil.rmtree(work)
    (work / "config").mkdir(parents=True)
    (work / "vault").mkdir()
    env = dict(os.environ, XDG_CONFIG_HOME=str(work / "config"), FM_UI_DIST="ui/dist",
               # Firefox kept between runs, checked against Mozilla's signed sums each time (`browser_kit.rs`).
               FM_BROWSER_CACHE=str(CACHE / "firefox-archives"),
               FM_VAULT=str(work / "vault"), FM_VAULTS=str(work / "vaults.json"), FM_OPEN="0",
               FM_AUTO_SHUTDOWN="0", FM_ADDR=f"127.0.0.1:{PORT}")
    server = subprocess.Popen([str(ROOT / "target" / "debug" / "fm-serve")], cwd=ROOT, env=env,
                              stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        wait_until_up()
        # A vault with a git identity, so the app shows its notes rather than its first-run welcome.
        post("/api/set_identity", {"name": "Check Reader", "email": "reader@example.org"})
        post("/api/browser_setup")
        for _ in range(600):
            s = post("/api/browser_status")
            if not s.get("busy"):
                break
            time.sleep(1)
        if s.get("error") or not s.get("installed"):
            fail(f"setup: {s.get('error')}")
        print(f"browser-check: set up {s.get('version')} (pinned {s.get('pinned')})")
        # **Measured with other switches** (`privacy-plan-2026-11-19.md` 5.2): `FM_CHECK_PRIVACY` is
        # a change to this device's switches, e.g. `{"ads": true}`, applied before the starts.
        if os.environ.get("FM_CHECK_PRIVACY"):
            post("/api/set_privacy", json.loads(os.environ["FM_CHECK_PRIVACY"]))
            print(f"browser-check: privacy switches {os.environ['FM_CHECK_PRIVACY']}")
        browser = work / "config" / "formicaria" / "browser"
        ff = browser / "firefox"

        # **The two browsers measured in turn** (2026-11-17): formicaria's five runs used to come first and
        # the unmodified five minutes later, so a machine busy at the start and quiet at the end read as
        # formicaria starting slower (twice in a day, 1.4 s against 0.8 s, with the kit unchanged).
        # Alternating them puts both under the same load.
        base_ff = stock_esr(version, sha)
        # `FM_CHECK_ONLY=sites`: only the sites, while working on what a page shows.
        if os.environ.get("FM_CHECK_ONLY") == "sites":
            ok, why = sites_as_firefox(ff, base_ff, work)
            print(f"browser-check: {'ok  ' if ok else 'FAIL'} no site shows a warning that Firefox does not" + ("" if ok else f" — {why}"))
            sys.exit(0 if ok else 1)
        # `FM_CHECK_ONLY=settings`: only the Settings page's walk (bring in, ☆, tabs), while working on it.
        if os.environ.get("FM_CHECK_ONLY") == "settings":
            testpage = serve_test_page(work)
            try:
                page = settings_page(ff, work)
            finally:
                testpage.terminate()
            print(f"browser-check: the walk read {json.dumps(page)[:1500]}")
            sys.exit(0 if all(v is True for k, v in page.items() if k.startswith("star_")) else 1)
        kit, base = [], []
        for i in range(RUNS):
            kit.append(measure(ff, work / f"kit-profile-{i}", work / f"kit-{i}.json", [f"{SERVER}/f/group/Library", *PAGES], {}, warm=True))
            base.append(measure(base_ff, work / f"stock-profile-{i}", work / "stock.json", [SERVER, *PAGES], {}, warm=True))
        first = kit[0]
        # Each start's memory beside what it had installed by then: the privacy extensions install at
        # the first start, and how far they got moves the figure (2026-11-18).
        for i, k in enumerate(kit):
            print(f"browser-check:      start {i + 1}: {k.get('pss_kb', 0) / 1024:.0f} MB, extensions "
                  f"{(k.get('privacy') or {}).get('addons')}; unmodified {base[i].get('pss_kb', 0) / 1024:.0f} MB")
        checks = {
            "the window is titled formicaria": str(first.get("title", "")).endswith("formicaria")
            and "Firefox" not in str(first.get("title", "")),
            "the brand strings are formicaria's": first.get("brand") == "formicaria",
            "the ant is first in the tab row": first.get("ant") == "TabsToolbar" and first.get("antFirst") is True,
            # Home since the shell (2026-11-22).
            "the browser opens on Home": str(first.get("home", "")).endswith("/f/home"),
            "only the add-on line is locked": first.get("signaturesLocked") is True,
            "Sync and the Mozilla account are available": first.get("accountsAvailable") is True,
            "split view is on": first.get("splitView") is True,
            "tabs are classic, on top, in view": first.get("tabsOnTop") is True,
            "scrolling over the tabs switches them": first.get("scrollSwitches") is True,
            "the sidebar's button is gone": first.get("sidebarButtonGone") is True,
            # Settings at the right end of the tab row, on every tab (`formicaria.cfg`, `makeSettings`).
            "Settings is at the right end of the tab row": (first.get("settings") or {}).get("area") == "TabsToolbar"
            and (first.get("settings") or {}).get("drawn") is True
            and (first.get("settings") or {}).get("rightOfTabs") is True,
            # The Privacy sheet's switches reach Firefox (2026-11-17): read and applied at start.
            "the privacy switches are applied": (first.get("privacy") or {}).get("v", -1) >= 0
            and (first.get("privacy") or {}).get("clean") is True
            and (first.get("privacy") or {}).get("engine") == "DuckDuckGo",
            "Mozilla's telemetry is off by policy": "DisableTelemetry" in first.get("policies", []),
            "Firefox's updater is off by policy": "DisableAppUpdate" in first.get("policies", []),
            "the Mozilla account is not disabled": "DisableFirefoxAccounts" not in first.get("policies", []),
            "the startup script raised nothing": not first.get("errors"),
        }
        sites_ok, sites_why = sites_as_firefox(ff, base_ff, work)
        checks["no site shows a warning that Firefox does not" + ("" if sites_ok else f" — {sites_why}")] = sites_ok
        testpage = serve_test_page(work)
        try:
            page = settings_page(ff, work)
        finally:
            testpage.terminate()
        checks.update({
            "Settings: no 'managed by your organization'": page.get("managed_notice_hidden") is True,
            "Settings: no 'new look' promotion": page.get("new_look_promo_hidden") is True,
            "Settings: no 'AI controls' section": page.get("ai_controls_hidden") is True,
            "Settings: no link to Mozilla's support": page.get("support_link_hidden") is True,
            "Settings: 'About formicaria' is there": page.get("about_formicaria") is True,
            "Settings: formicaria's mark instead of Firefox's mascot": page.get("mascot_is_formicarias") is True,
            "a link a page opens becomes a tab of this window": page.get("links_are_tabs") is True,
            "a sign-in popup holds only the page the site opened": page.get("popup_is_its_page") is True,
            "the download question's window holds only the question": page.get("download_question_alone") is True,
            "bring in: a passage arrives quoted, with its source": page.get("bring_text") is True,
            "bring in: an image arrives kept in the vault": page.get("bring_image") is True,
            "bring in: a table arrives as a table": page.get("bring_table") is True,
            "bring in: the icon after a selection opens the chooser, and Enter brings it into the note in front": page.get("bring_chip") is True,
            "snip: an area brings its text, image and table in, each as itself": page.get("snip") is True,
            "the page is then found in your notes": page.get("page_in_notes") is True,
            "☆ makes a bookmark note as well as Firefox's bookmark": page.get("star_note") is True,
            "☆'s panel says the note is in formicaria, with a link to it": page.get("star_panel_says") is True,
            "☆ pressed again makes no second note": page.get("star_again_no_copy") is True,
            "Remove in ☆'s panel takes back the note and the bookmark": page.get("star_remove_both") is True,
            "a feature is a page in a tab of its own, named by what it shows": page.get("feature_tab") is True,
            "a note is a page in a tab of its own": page.get("note_tab") is True,
            "two of formicaria's tabs share the window in split view": page.get("split_formicaria_tabs") is True,
            "the quick switch returns to the tab before, whatever it holds": page.get("quick_switch") is True,
            "+ opens formicaria's new tab, suggesting the features": page.get("plus_suggests") is True,
            "one row on formicaria's tabs; the address row on web pages": page.get("one_row") is True,
            "the old whole-app page in a tab becomes Notes": page.get("whole_app_to_notes") is True,
            "tabs from earlier versions become their items' pages": page.get("old_tabs_carried") is True,
            "the toolbar is bare: the address bar, no home or account button": page.get("bare_toolbar") is True,
        })
        avg = lambda xs, k: sum(x.get(k) or 0 for x in xs) / len(xs)
        # **What a feature tab costs** (`decisions.md` 2026-11-17: at most 20 MB each, against the
        # 35 MB a copy of the whole app measured per tab): one feature page against five, twice each.
        # Nine against one (2026-11-17): with five, the ±50 MB a whole browser wobbles by between runs was
        # divided by four and read 12 to 37 MB per tab for the same pages; nine divides it by eight.
        feats = [f"{SERVER}/f/group/Library", f"{SERVER}/f/group/Tasks", f"{SERVER}/f/agenda",
                 f"{SERVER}/f/group/Food", f"{SERVER}/f/timeline", f"{SERVER}/f/group/Fitness",
                 f"{SERVER}/f/group/Health", f"{SERVER}/f/group/Music", f"{SERVER}/f/board"]
        # In turn, three of each, and the median: two runs of each, one after the other, read the
        # machine's load as a change of 4.5 MB per tab between two runs of the same pages.
        one, five = [], []
        for i in range(3):
            one.append(measure(ff, work / f"feat1-{i}", work / "feat1.json", feats[:1], {}))
            five.append(measure(ff, work / f"feat9-{i}", work / "feat9.json", feats, {}))
        med = lambda xs: sorted(x.get("pss_kb") or 0 for x in xs)[len(xs) // 2] / 1024
        per_tab = (med(five) - med(one)) / 8
        print(f"browser-check: memory  one feature tab {med(one):.0f} MB, nine {med(five):.0f} MB (medians of 3): {per_tab:.1f} MB per extra tab")
        checks["each extra feature tab costs at most 20 MB"] = per_tab <= 20
        for name, ok in checks.items():
            print(f"browser-check: {'ok  ' if ok else 'FAIL'} {name}")
        if not checks.get("the privacy switches are applied", True):
            # What each start reported, so a failure says whether it was late or wrong.
            print("browser-check:      privacy per start: " + "; ".join(json.dumps(k.get("privacy")) for k in kit))

        # Every run, not only the averages: a slow first run (a cold disk, a busy machine) shows as one.
        for name, runs in (("formicaria", kit), ("unmodified", base)):
            print(f"browser-check: runs   {name}: " + ", ".join(
                f"{(r.get('sessionRestored_ms') or 0):.0f} ms/{(r.get('pss_kb') or 0) / 1024:.0f} MB" for r in runs))
        km, bm = avg(kit, "pss_kb") / 1024, avg(base, "pss_kb") / 1024
        ks, bs = avg(kit, "sessionRestored_ms"), avg(base, "sessionRestored_ms")
        print(f"browser-check: memory  formicaria {km:.0f} MB  vs unmodified {bm:.0f} MB  ({(km / bm - 1) * 100:+.1f} %)")
        print(f"browser-check: start   formicaria {ks:.0f} ms  vs unmodified {bs:.0f} ms")
        budget_ok = km <= bm * MEMORY_BUDGET and ks <= bs * START_BUDGET + START_NOISE_MS
        print(f"browser-check: {'ok  ' if budget_ok else 'FAIL'} within the budgets (memory +10 %, start +15 % + 100 ms)")
        (CACHE / "last.json").write_text(json.dumps({"version": version, "checks": checks,
                                                     "memory_mb": [km, bm], "start_ms": [ks, bs]}, indent=1))
        if not all(checks.values()) or not budget_ok:
            fail("see above")
        print("browser-check: all good")
    finally:
        stop(work / "config" / "formicaria" / "browser" / "firefox")
        server.terminate()


if __name__ == "__main__":
    main()
