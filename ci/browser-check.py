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
        # **Add to the open note**, end to end: a note open in the panel, a selection on a page,
        # Alt+Shift+A, and the passage is in the note on disk with where it came from.
        try:
            made = post("/api/create_notes", {"notes": [{"title": "Reading", "body": "Start."}]})
            nid = made[0]["id"]
            m.cmd("WebDriver:ExecuteScript", {"args": [SERVER + "/#note=" + nid], "script": """
                document.getElementById('formicaria-browser').fixupAndLoadURIString(arguments[0],
                  { triggeringPrincipal: Services.scriptSecurityManager.getSystemPrincipal() });
                const t = gBrowser.addTab('https://example.org/', { triggeringPrincipal: Services.scriptSecurityManager.getSystemPrincipal() });
                gBrowser.selectedTab = t;"""})
            time.sleep(6)
            handles = m.cmd("WebDriver:GetWindowHandles")
            handles = handles if isinstance(handles, list) else handles.get("value", handles)
            m.cmd("WebDriver:SwitchToWindow", {"handle": handles[-1]})
            m.cmd("Marionette:SetContext", {"value": "content"})
            m.cmd("WebDriver:ExecuteScript", {"args": [], "script": """
                const r = document.createRange(); r.selectNodeContents(document.querySelector('h1') || document.body);
                getSelection().removeAllRanges(); getSelection().addRange(r);"""})
            # The add-on's Alt+Shift+A, by its own command: the harness's synthetic keys do not reach
            # add-on shortcuts, and this runs exactly what the key runs.
            m.cmd("Marionette:SetContext", {"value": "chrome"})
            m.cmd("WebDriver:ExecuteScript", {"args": [], "script": """
                [...document.querySelectorAll('key')].find(k => k.getAttribute('modifiers') === 'alt,shift' && k.getAttribute('key') === 'A').doCommand();"""})
            time.sleep(8)
            body = post("/api/get", {"id": nid}).get("body", "")
            seen["add_to_note"] = body.startswith("Start.\n\n> ") and "> — [Example Domain](https://example.org/)" in body
            if not seen["add_to_note"]:
                print(f"browser-check: the note read {body!r}")
            found = post("/api/page_notes", {"url": "https://example.org"})
            seen["page_in_notes"] = any(n.get("id") == nid for n in found)
            m.cmd("Marionette:SetContext", {"value": "chrome"})
        except Exception as e:  # noqa: BLE001 - reported as a failed check
            print(f"browser-check: add to note: {e}")
            seen.setdefault("add_to_note", False)
            seen.setdefault("page_in_notes", False)
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
            "Mozilla's telemetry is off by policy": "DisableTelemetry" in first.get("policies", []),
            "Firefox's updater is off by policy": "DisableAppUpdate" in first.get("policies", []),
            "the Mozilla account is not disabled": "DisableFirefoxAccounts" not in first.get("policies", []),
            "the startup script raised nothing": not first.get("errors"),
            "no tab holds formicaria": first.get("formicariaTabs") == 0,
        }
        page = settings_page(ff, work)
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
            "the add-on adds a selection to the note open in the panel": page.get("add_to_note") is True,
            "the page is then found in your notes": page.get("page_in_notes") is True,
        })
        for name, ok in checks.items():
            print(f"browser-check: {'ok  ' if ok else 'FAIL'} {name}")

        base_ff = stock_esr(version, sha)
        base = [measure(base_ff, work / f"stock-profile-{i}", work / "stock.json", [SERVER, *PAGES], {})
                for i in range(RUNS)]
        avg = lambda xs, k: sum(x.get(k) or 0 for x in xs) / len(xs)
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
