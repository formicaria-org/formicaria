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
            "the formicaria tab is pinned": first.get("pinned") is True,
            "only the add-on line is locked": first.get("signaturesLocked") is True,
            "Sync and the Mozilla account are available": first.get("accountsAvailable") is True,
            "split view is on": first.get("splitView") is True,
            "Mozilla's telemetry is off by policy": "DisableTelemetry" in first.get("policies", []),
            "Firefox's updater is off by policy": "DisableAppUpdate" in first.get("policies", []),
            "the Mozilla account is not disabled": "DisableFirefoxAccounts" not in first.get("policies", []),
            "the startup script raised nothing": not first.get("errors"),
        }
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
