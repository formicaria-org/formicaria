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
RUNS = 5  # five, not three (2026-11-16): the memory margin is close enough that noise matters

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


def measure(firefox: Path, profile: Path, report: Path, urls: list[str], extra_env: dict) -> dict:
    """Start `firefox` headless on `urls`, wait, read its report and its memory, stop it."""
    if profile.exists():
        shutil.rmtree(profile)
    profile.mkdir(parents=True)
    report.unlink(missing_ok=True)
    env = dict(os.environ, FM_BROWSER_CHECK=str(report), **extra_env)
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
    m.cmd("WebDriver:ExecuteScript", {"args": [SERVER + "/#note=" + nid, TEST_PAGE], "script": """
        const sp = Services.scriptSecurityManager.getSystemPrincipal();
        document.getElementById('formicaria-browser').fixupAndLoadURIString(arguments[0], { triggeringPrincipal: sp });
        gBrowser.selectedTab = gBrowser.addTab(arguments[1], { triggeringPrincipal: sp });"""})
    time.sleep(6)
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
    # The panel moves below the page, then to its left (⇆ steps right, below, left, above), and
    # formicaria is still in it each time.
    m.cmd("Marionette:SetContext", {"value": "chrome"})
    side = m.cmd("WebDriver:ExecuteAsyncScript", {"args": [SERVER], "script": """
        const done = arguments[arguments.length - 1];
        const press = () => document.getElementById('formicaria-panel-side').doCommand();
        const look = () => {
          const p = document.getElementById('formicaria-panel').getBoundingClientRect();
          const t = document.getElementById('tabbrowser-tabpanels').getBoundingClientRect();
          return { side: document.getElementById('browser').getAttribute('formicaria-side'),
                   below: p.top >= t.bottom - 1, left: p.right <= t.left + 1,
                   ours: (document.getElementById('formicaria-browser').currentURI?.spec || '').startsWith(arguments[0]) };
        };
        press();
        setTimeout(() => {
          const bottom = look();
          press();
          setTimeout(() => {
            const left = look();
            press(); press();
            done({ bottom, left });
          }, 3000);
        }, 3000);"""})["value"]
    b, l = side.get("bottom", {}), side.get("left", {})
    out["panel_bottom"] = b.get("side") == "bottom" and b.get("below") is True and b.get("ours") is True
    out["panel_left"] = l.get("side") == "left" and l.get("left") is True and l.get("ours") is True
    return out


def lean_tabs(m: "Marionette") -> dict:
    """A study room as a lean tab of the collection (workspace-design §11, D1)."""
    nid = post("/api/create_notes", {"notes": [{"title": "Lean room", "body": "Here."}]})[0]["id"]
    m.cmd("Marionette:SetContext", {"value": "chrome"})
    r = m.cmd("WebDriver:ExecuteAsyncScript", {"args": [SERVER, nid], "scriptTimeout": 60000, "script": """
        const done = arguments[arguments.length - 1];
        const [server, id] = [arguments[0], arguments[1]];
        const sp = Services.scriptSecurityManager.getSystemPrincipal();
        const wait = (ms) => new Promise(r => setTimeout(r, ms));
        (async () => {
          gBrowser.addTab(server + '/#tab=' + encodeURIComponent('note:' + id) + '&title=Lean%20room', { triggeringPrincipal: sp });
          await wait(4000);
          const lean = gBrowser.tabs.find(t => t.getAttribute('formicaria-item') === 'note:' + id);
          const out = { made: !!lean, label: lean?.label, selected: gBrowser.selectedTab === lean,
                        full: document.getElementById('browser').getAttribute('formicaria'),
                        light: !lean?.linkedBrowser?.currentURI || lean.linkedBrowser.currentURI.spec === 'about:blank' };
          const web = gBrowser.tabs.find(t => /^https?:/.test(t.linkedBrowser.currentURI.spec) && !t.getAttribute('formicaria-item'));
          if (web) { gBrowser.selectedTab = web; await wait(1000); out.beside = document.getElementById('browser').getAttribute('formicaria'); }
          gBrowser.selectedTab = lean; await wait(1500);
          out.again = document.getElementById('browser').getAttribute('formicaria');
          out.labelAfter = lean.label;
          try { out.kept = JSON.parse(SessionStore.getTabState(lean)).extData?.['formicaria-item'] === 'note:' + id; } catch (e) { out.kept = String(e); }
          // **The tab follows formicaria's item**: the app saying it moved to the board moves this tab.
          const panel = document.getElementById('formicaria-browser');
          panel.fixupAndLoadURIString(server + '/#at=' + encodeURIComponent('view:board') + '&title=Board', { triggeringPrincipal: sp });
          await wait(1500);
          out.followed = lean.getAttribute('formicaria-item');
          out.followedLabel = lean.label;
          // **+ opens formicaria's Open...** in a new formicaria tab, which fills the window.
          const before = gBrowser.tabs.length;
          BrowserCommands.openTab();
          await wait(2500);
          const nt = gBrowser.selectedTab;
          out.plus = { more: gBrowser.tabs.length === before + 1, item: nt.getAttribute('formicaria-item'),
                       state: document.getElementById('browser').getAttribute('formicaria') };
          gBrowser.removeTab(nt);
          done(out);
        })().catch(e => done({ error: String(e) }));"""})["value"]
    ok = (r.get("made") is True and r.get("selected") is True and r.get("full") == "full"
          and r.get("label") == "Lean room" and r.get("labelAfter") == "Lean room" and r.get("light") is True)
    plus = r.get("plus") or {}
    out = {"lean_tab": ok, "lean_kept": r.get("kept") is True,
           "lean_fills": r.get("beside") == "folded" and r.get("again") == "full",
           "tab_follows": r.get("followed") == "view:board" and r.get("followedLabel") == "Board",
           "plus_opens": plus.get("more") is True and plus.get("item") == "open:" and plus.get("state") == "full"}
    if not all(out.values()):
        print(f"browser-check: lean tab seen {r}")
    return out


def pairs(m: "Marionette") -> dict:
    """D3: a tab dropped beside the one shown (workspace-design §12), by the function a drop calls."""
    ids = [n["id"] for n in post("/api/create_notes", {"notes": [{"title": "Room A", "body": "a"}, {"title": "Room B", "body": "b"}]})]
    m.cmd("Marionette:SetContext", {"value": "chrome"})
    r = m.cmd("WebDriver:ExecuteAsyncScript", {"args": [SERVER, ids], "scriptTimeout": 60000, "script": """
        const done = arguments[arguments.length - 1];
        const [server, ids] = [arguments[0], arguments[1]];
        const sp = Services.scriptSecurityManager.getSystemPrincipal();
        const wait = (ms) => new Promise(r => setTimeout(r, ms));
        const state = () => document.getElementById('browser').getAttribute('formicaria');
        (async () => {
          const out = {};
          const p1 = gBrowser.addTab('https://example.org/?one', { triggeringPrincipal: sp });
          const p2 = gBrowser.addTab('https://example.org/?two', { triggeringPrincipal: sp });
          await wait(2500);
          gBrowser.selectedTab = p1; await wait(500);
          out.pages = window.formicariaPair(p2, 'left');
          out.split = !!p1.splitview && !!p2.splitview;
          for (const id of ids) gBrowser.addTab(server + '/#tab=' + encodeURIComponent('note:' + id) + '&title=R', { triggeringPrincipal: sp });
          await wait(4000);
          const rooms = gBrowser.tabs.filter(t => ids.some(id => t.getAttribute('formicaria-item') === 'note:' + id));
          const page = gBrowser.tabs.find(t => t.linkedBrowser.currentURI.spec.includes('example.org') && !t.splitview);
          gBrowser.selectedTab = page || p1; await wait(800);
          out.mixed = window.formicariaPair(rooms[0], 'left');
          out.mixedSide = document.getElementById('browser').getAttribute('formicaria-side');
          out.mixedState = state();
          gBrowser.selectedTab = rooms[0]; await wait(1500);
          out.both = window.formicariaPair(rooms[1], 'right');
          out.bothState = state();
          window.formicariaPair(null, 'right');
          document.getElementById('browser').setAttribute('formicaria-side', 'right');
          // The quick switch (Alt+Shift+X): back to the item chosen before, whatever its kind.
          const before = gBrowser.selectedTab;
          const other = gBrowser.tabs.find(t => t !== before && !t.closing);
          gBrowser.selectedTab = other; await wait(400);
          out.swap = window.formicariaSwap();
          out.swapBack = gBrowser.selectedTab === before;
          // The floating mark: shown only while that side is closed.
          const fl = document.getElementById('formicaria-float');
          document.getElementById('browser').setAttribute('formicaria', 'folded'); await wait(200);
          out.floatShown = !!fl && getComputedStyle(fl).display !== 'none';
          document.getElementById('browser').setAttribute('formicaria', 'beside'); await wait(200);
          out.floatHidden = !!fl && getComputedStyle(fl).display === 'none';
          // Firefox kept bare: no home or account button on the toolbar (the tab row and address bar stay).
          const gone = id => { const b = document.getElementById(id); return !b || getComputedStyle(b).display === 'none'; };
          out.bare = gone('home-button') && gone('fxa-toolbar-menu-button') && !!document.getElementById('urlbar');
          done(out);
        })().catch(e => done({ error: String(e) }));"""})["value"]
    ok = {"pair_pages": r.get("pages") == "split" and r.get("split") is True,
          "pair_room_page": r.get("mixed") == "beside" and r.get("mixedSide") == "left" and r.get("mixedState") == "beside",
          "pair_rooms": r.get("both") == "rooms" and r.get("bothState") == "full",
          "quick_switch": r.get("swap") == "tab" and r.get("swapBack") is True,
          "float_when_closed": r.get("floatShown") is True and r.get("floatHidden") is True,
          "bare_toolbar": r.get("bare") is True}
    if not all(ok.values()):
        print(f"browser-check: pairs seen {r}")
    return ok


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
        # The panel: formicaria's address opened in a tab goes to the panel (a note's address too) and
        # the tab goes away; a link formicaria opens in a new window becomes a tab of this window.
        tabs = m.cmd("WebDriver:ExecuteAsyncScript", {"args": [SERVER], "scriptTimeout": 60000, "script": """
            const done = arguments[arguments.length - 1];
            const url = arguments[0];
            const gb = window.gBrowser;
            const sp = Services.scriptSecurityManager.getSystemPrincipal();
            const wait = (ms) => new Promise(r => setTimeout(r, ms));
            const ours = () => gb.tabs.filter(t => (t.linkedBrowser.currentURI.spec || '').startsWith(url)).length;
            const panel = () => document.getElementById('formicaria-browser');
            const state = () => document.getElementById('browser').getAttribute('formicaria');
            (async () => {
              gb.addTab('https://example.org/', { triggeringPrincipal: sp });
              gb.addTab(url, { triggeringPrincipal: sp }); await wait(5000);
              const r = { first: (panel()?.currentURI?.spec || '').startsWith(url), tabs1: ours(), state1: state() };
              gb.addTab(url + '/#note=check', { triggeringPrincipal: sp }); await wait(5000);
              r.note = panel()?.currentURI?.spec || ''; r.tabs2 = ours();
              const n = gb.tabs.length;
              panel().fixupAndLoadURIString('data:text/html,<a id=a target=_blank href="https://example.org/?from=panel">x</a><script>setTimeout(()=>document.getElementById("a").click(),300)</script>', { triggeringPrincipal: sp });
              await wait(4000);
              r.linkTab = gb.tabs.some(t => t.linkedBrowser.currentURI.spec.includes('from=panel'));
              r.windows = [...Services.wm.getEnumerator('navigator:browser')].length;
              document.getElementById('formicaria-panel-fold').doCommand(); await wait(300);
              r.folded = state();
              done(r);
            })().catch(e => done({ error: String(e) }));"""})["value"]
        seen["panel_takes_address"] = tabs.get("first") is True and tabs.get("tabs1") == 0
        seen["panel_takes_note"] = str(tabs.get("note", "")).endswith("#note=check") and tabs.get("tabs2") == 0
        seen["panel_links_are_tabs"] = tabs.get("linkTab") is True and tabs.get("windows") == 1
        seen["panel_folds"] = tabs.get("folded") == "folded"
        if not all(seen[k] for k in ("panel_takes_address", "panel_takes_note", "panel_links_are_tabs", "panel_folds")):
            print(f"browser-check: panel seen {tabs}")
        # **Bring in**, end to end (research §11): a note open in the panel, then from a page each kind
        # of thing, and each must arrive in the note on disk as itself, with where it came from.
        try:
            seen.update(bring_in(m))
        except Exception as e:  # noqa: BLE001 - reported as failed checks
            print(f"browser-check: bring in: {e}")
        try:
            seen.update(lean_tabs(m))
        except Exception as e:  # noqa: BLE001 - reported as failed checks
            print(f"browser-check: lean tabs: {e}")
        try:
            seen.update(pairs(m))
        except Exception as e:  # noqa: BLE001 - reported as failed checks
            print(f"browser-check: pairs: {e}")
        for k in ("bring_text", "bring_image", "bring_table", "bring_chip", "snip", "page_in_notes", "panel_left", "panel_bottom",
                  "lean_tab", "lean_kept", "lean_fills", "tab_follows", "plus_opens", "pair_pages", "pair_room_page", "pair_rooms",
                  "quick_switch", "float_when_closed", "bare_toolbar"):
            seen.setdefault(k, False)
        m.cmd("WebDriver:DeleteSession")
        return seen
    finally:
        stop(ff)


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
               FM_VAULT=str(work / "vault"), FM_VAULTS=str(work / "vaults.json"), FM_OPEN="0",
               FM_AUTO_SHUTDOWN="0", FM_ADDR=f"127.0.0.1:{PORT}")
    server = subprocess.Popen([str(ROOT / "target" / "debug" / "fm-serve")], cwd=ROOT, env=env,
                              stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        time.sleep(2)
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
        browser = work / "config" / "formicaria" / "browser"
        ff = browser / "firefox"

        kit = [measure(ff, work / f"kit-profile-{i}", work / "kit.json", [SERVER, *PAGES], {})
               for i in range(RUNS)]
        first = kit[0]
        checks = {
            "the window is titled formicaria": str(first.get("title", "")).endswith("formicaria")
            and "Firefox" not in str(first.get("title", "")),
            "the brand strings are formicaria's": first.get("brand") == "formicaria",
            "the formicaria button is in the toolbar": first.get("widget") is True,
            "formicaria is in its panel": first.get("panel") is True,
            "the panel keeps formicaria's own storage": first.get("partitionKey") == "",
            "only the add-on line is locked": first.get("signaturesLocked") is True,
            "Sync and the Mozilla account are available": first.get("accountsAvailable") is True,
            "split view is on": first.get("splitView") is True,
            "tabs are classic, on top, in view": first.get("tabsOnTop") is True,
            "scrolling over the tabs switches them": first.get("scrollSwitches") is True,
            "the sidebar's button is gone": first.get("sidebarButtonGone") is True,
            "Mozilla's telemetry is off by policy": "DisableTelemetry" in first.get("policies", []),
            "Firefox's updater is off by policy": "DisableAppUpdate" in first.get("policies", []),
            "the Mozilla account is not disabled": "DisableFirefoxAccounts" not in first.get("policies", []),
            "the startup script raised nothing": not first.get("errors"),
            "no tab holds formicaria": first.get("formicariaTabs") == 0,
        }
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
            "the panel: formicaria's address opened in a tab goes to it": page.get("panel_takes_address") is True,
            "the panel: a note's address opens in it": page.get("panel_takes_note") is True,
            "the panel: a link it opens becomes a tab of the window": page.get("panel_links_are_tabs") is True,
            "the panel: it folds away": page.get("panel_folds") is True,
            "bring in: a passage arrives quoted, with its source": page.get("bring_text") is True,
            "bring in: an image arrives kept in the vault": page.get("bring_image") is True,
            "bring in: a table arrives as a table": page.get("bring_table") is True,
            "bring in: the icon after a selection opens the chooser, and Enter brings it into the note in front": page.get("bring_chip") is True,
            "snip: an area brings its text, image and table in, each as itself": page.get("snip") is True,
            "the page is then found in your notes": page.get("page_in_notes") is True,
            "the panel moves below the page, formicaria still in it": page.get("panel_bottom") is True,
            "a study room opens as a lean tab, formicaria filling the window": page.get("lean_tab") is True,
            "the lean tab is kept by the session (back after a restart)": page.get("lean_kept") is True,
            "one item fills the window: a page chosen shows the page alone; formicaria's tab fills again": page.get("lean_fills") is True,
            "formicaria's tab follows the item formicaria moves to": page.get("tab_follows") is True,
            "+ opens formicaria's Open... in a new formicaria tab": page.get("plus_opens") is True,
            "D3: a page dropped beside a page makes a split view": page.get("pair_pages") is True,
            "D3: a study room dropped beside a page puts formicaria on that side": page.get("pair_room_page") is True,
            "D3: two study rooms dropped together show side by side": page.get("pair_rooms") is True,
            "the quick switch returns to the item before, any kind": page.get("quick_switch") is True,
            "a small mark shows what is beside, only when that side is closed": page.get("float_when_closed") is True,
            "the toolbar is bare: the address bar, no home or account button": page.get("bare_toolbar") is True,
            "the panel moves to the left, formicaria still in it": page.get("panel_left") is True,
        })
        for name, ok in checks.items():
            print(f"browser-check: {'ok  ' if ok else 'FAIL'} {name}")

        base_ff = stock_esr(version, sha)
        base = [measure(base_ff, work / f"stock-profile-{i}", work / "stock.json", [SERVER, *PAGES], {})
                for i in range(RUNS)]
        avg = lambda xs, k: sum(x.get(k) or 0 for x in xs) / len(xs)
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
