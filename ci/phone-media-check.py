#!/usr/bin/env python3
"""**Save media on the phone, through the phone app, in an Android emulator** (2026-11-17; the owner,
after 0.51's phone could save nothing from YouTube or Bandcamp: *"you can try it out here on the
computer no? … even have tests for it in the ci"*, *"check inside the app, not only stand alone"*,
*"Wikipedia is perfect"*, *"include instances where there are many contents in a single page"*).

The phone's runner is Android code (`MediaRun.kt`: yt-dlp and Python inside the app, a video's two
streams joined by Android's muxer) over the core (`media_args`, `media_keep`: MP3 and tags by
`fm-audio`, filing). This installs the phone app (x86_64, debug) on an emulator on this computer —
**never a phone**: the device must say it is an emulator — and does what a person does, through the
app's own screens: **+**, the address, **≡ → Save media**, the choice, **Save**; then reads the note
it made and the file's first bytes.

**The release build, as people get it** (2026-11-18: 0.54's debug build passed here while the release
on a phone could not save media). By default the APK is `android-release`'s own build, for x86_64:
shrunk by R8, optimised, with the WebView's debugging on (`tauri/devtools`) so this can drive it, and
signed with a key made for the check. `FM_PHONE_BUILD=debug` uses the debug build instead.

The pages are the computer's (`ci/usability-check.py`): Wikipedia's *Ode to Joy* (one recording, as
MP3), *Toccata and Fugue in D minor* (four recordings: the third kept, and checked to be the third),
Commons' *Ant Bed With Ants* (a short video); and a clip served from this computer whose picture
and sound come apart (DASH, as YouTube serves them), so the phone's joining is used.

Run it capped (the laptop froze once under an uncapped run), one heavy run at a time:
`systemd-run --user --scope -p MemoryMax=9G -p CPUQuota=1200% -- pixi run -e android phone-media-check`.
Needs /dev/kvm and the emulator (`.android/sdk`, the `formicaria` device: `pixi run -e android
android-avd`). `FM_PHONE_REBUILD=0` reuses the last build. Exits 1 on a failure, saying which.
"""
import base64
import functools
import http.server
import json
import os
import shutil
import socket
import socketserver
import struct
import subprocess
import sys
import threading
import time
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SDK = ROOT / ".android" / "sdk"
ADB = str(ROOT / ".android" / "platform-tools" / "adb")
EMU = str(SDK / "emulator" / "emulator")
SERIAL = "emulator-5554"
PKG = "dev.formicaria.notes"
WORK = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "formicaria-phone-media-check"
BUILD = os.environ.get("FM_PHONE_BUILD", "release")
APK = (ROOT / "mobile/src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk"
       if BUILD == "debug" else ROOT / "mobile/formicaria-check-android-x86_64.apk")
fails: list[str] = []
# How the app's own files are read: `run-as` for a debug build; a release build allows it no one, so
# there the emulator's root reads them (`adb root`, which an emulator's image allows; a phone's not).
AS_APP = ["run-as", PKG] if BUILD == "debug" else []

# What a person keeps, page by page: the page, the candidate's label starts with, which of those,
# how many it must list at least, what to keep, where it goes.
CASES = [
    ("Ode to Joy: its recording, as MP3", "https://en.wikipedia.org/wiki/Ode_to_Joy", "Audio:", 0, 1, "mp3", "Music"),
    ("Toccata and Fugue: the second of its recordings, as served",
     "https://en.wikipedia.org/wiki/Toccata_and_Fugue_in_D_minor,_BWV_565", "Audio:", 1, 4, "audio", "Music"),
    ("Ant Bed With Ants (Commons): the video", "https://commons.wikimedia.org/wiki/File:Ant_Bed_With_Ants.webm",
     "Video:", 0, 1, "video", "Videos"),
    ("a clip whose picture and sound come apart (DASH): the video", "http://10.0.2.2:{port}/index.html",
     "Video:", 0, 1, "video", "Videos"),
    # **YouTube, as people use it** (the owner on 0.54, 2026-11-18: *"download of multimedia does not
    # work (tried it on the phone)"*): its picture and sound always come apart, and its extraction
    # needs yt-dlp's JavaScript runtime (QuickJS, in the app). The first video ever posted: 19 s, up
    # since 2005.
    ("YouTube: the video", "https://www.youtube.com/watch?v=jNQXAC9IVRw", "", 0, 1, "video", "Videos"),
    ("YouTube: only the sound, as MP3", "https://www.youtube.com/watch?v=jNQXAC9IVRw", "", 0, 1, "mp3", "Music"),
]


def say(msg: str) -> None:
    print(f"phone-media-check: {msg}", flush=True)


def check(what: str, ok: bool, why: str = "") -> None:
    say(f"{'ok  ' if ok else 'FAIL'} {what}{'' if ok else ' — ' + why}")
    if not ok:
        fails.append(what)


def adb(*args: str, check_rc: bool = True, timeout: int = 300) -> str:
    r = subprocess.run([ADB, "-s", SERIAL, *args], capture_output=True, text=True, timeout=timeout)
    if check_rc and r.returncode != 0:
        raise RuntimeError(f"adb {' '.join(args)}: {r.stderr.strip() or r.stdout.strip()}")
    return r.stdout


def fixtures() -> Path:
    """Three seconds of picture and sound as DASH (an H.264 stream and an AAC stream), on a page."""
    d = WORK / "site"
    if d.exists():
        shutil.rmtree(d)
    d.mkdir(parents=True)
    subprocess.run(
        ["ffmpeg", "-loglevel", "error", "-y", "-f", "lavfi", "-i", "testsrc=size=320x240:rate=25:duration=3",
         "-f", "lavfi", "-i", "sine=frequency=440:duration=3", "-map", "0", "-map", "1",
         "-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac", "-b:a", "96k",
         "-f", "dash", "-seg_duration", "1", "-use_template", "1", "-use_timeline", "0", str(d / "clip.mpd")],
        check=True,
    )
    (d / "index.html").write_text(
        '<!doctype html><title>A clip</title><h1>A clip</h1><video controls src="clip.mpd"></video>'
    )
    return d


def serve(d: Path) -> socketserver.TCPServer:
    handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=str(d))
    handler.log_message = lambda *a, **k: None  # type: ignore[assignment]
    # Any free port: a fixed one was still held by the last run's closing connections.
    srv = socketserver.ThreadingTCPServer(("127.0.0.1", 0), handler)
    srv.daemon_threads = True
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    return srv


def app_memory() -> str:
    """The app's memory as Android counts it (`dumpsys meminfo`'s total PSS): measured, not guessed
    (`lean-plan-2026-11-18.md` L3)."""
    out = adb("shell", "dumpsys", "meminfo", PKG, check_rc=False, timeout=60)
    for line in out.splitlines():
        parts = line.split()
        if parts[:2] == ["TOTAL", "PSS:"] and len(parts) > 2:
            return f"{int(parts[2]) / 1024:.0f} MB"
        if parts[:1] == ["TOTAL"] and len(parts) > 1 and parts[1].isdigit():
            return f"{int(parts[1]) / 1024:.0f} MB"
    return "unknown"


def signed(unsigned: Path) -> Path:
    """The release build signed with a key made for this check (never the release key)."""
    store = WORK / "check.keystore"
    java = Path(os.environ.get("CONDA_PREFIX", "")) / "lib" / "jvm" / "bin"
    if not store.exists():
        subprocess.run([str(java / "keytool"), "-genkeypair", "-keystore", str(store), "-alias", "check",
                        "-keyalg", "RSA", "-keysize", "2048", "-validity", "3650", "-storepass", "checkcheck",
                        "-keypass", "checkcheck", "-dname", "CN=phone-media-check"],
                       check=True, capture_output=True)
    bt = sorted((SDK / "build-tools").iterdir(), key=lambda d: [int(x) if x.isdigit() else 0 for x in d.name.split(".")])[-1]
    out = WORK / "check-signed.apk"
    env = dict(os.environ, JAVA_HOME=str(java.parent), PATH=f"{java}:{os.environ.get('PATH', '')}")
    subprocess.run([str(bt / "apksigner"), "sign", "--ks", str(store), "--ks-pass", "pass:checkcheck",
                    "--out", str(out), str(unsigned)], check=True, env=env)
    return out


def boot() -> subprocess.Popen:
    env = dict(os.environ, ANDROID_SDK_ROOT=str(SDK), ANDROID_HOME=str(SDK))
    # The device made by `pixi run -e android android-avd` (in the checkout), else the user's own.
    if (ROOT / ".android" / "avd" / "formicaria.ini").exists():
        env["ANDROID_AVD_HOME"] = str(ROOT / ".android" / "avd")
    image = SDK / "system-images" / "android-35" / "google_apis" / "x86_64"
    if not (image / "kernel-ranchu").exists():
        raise RuntimeError(f"the emulator's Android image is missing or incomplete ({image}): "
                           "sdkmanager \"system-images;android-35;google_apis;x86_64\"")
    # **The computer's GPU draws** (`-gpu host`; `FM_EMU_GPU=swiftshader_indirect` where there is
    # none): the software renderer crashed the emulator (SIGSEGV) on Commons' video page, again and
    # again, on 2026-11-17, with the host's video decoding off too. On the GPU all four cases passed.
    # (`-gpu guest` is not supported by this image.) Video is decoded inside Android.
    log = open(WORK / "emulator.log", "w")
    emu = subprocess.Popen(
        [EMU, "-avd", "formicaria", "-no-window", "-no-audio", "-no-boot-anim", "-no-snapshot",
         "-gpu", os.environ.get("FM_EMU_GPU", "host"), "-feature", "-HardwareDecoder", "-memory", "2048", "-cores", "4", "-port", "5554"],
        env=env, stdout=log, stderr=subprocess.STDOUT,
    )
    # Waits while the emulator runs; if it stops, says why (its log), not a timeout later.
    for _ in range(300):
        if emu.poll() is not None:
            tail = (WORK / "emulator.log").read_text(errors="replace").strip().splitlines()[-4:]
            raise RuntimeError("the emulator stopped: " + " | ".join(tail))
        if adb("get-state", check_rc=False).strip() == "device":
            break
        time.sleep(2)
    else:
        raise RuntimeError("the emulator did not appear to adb")
    for _ in range(300):
        if adb("shell", "getprop", "sys.boot_completed", check_rc=False).strip() == "1":
            break
        time.sleep(2)
    else:
        raise RuntimeError("the emulator did not finish starting")
    # **Never a phone.** Everything below installs and writes; it must be the emulator.
    if adb("shell", "getprop", "ro.kernel.qemu", check_rc=False).strip() != "1":
        raise RuntimeError(f"{SERIAL} is not an emulator: refusing")
    return emu


class Cdp:
    """A minimal Chrome DevTools client (a websocket by hand, no package), to one of the app's web views."""

    def __init__(self, ws_url: str):
        host, path = ws_url.removeprefix("ws://").split("/", 1)
        h, p = host.split(":")
        self.s = socket.create_connection((h, int(p)), timeout=1500)
        key = base64.b64encode(os.urandom(16)).decode()
        self.s.sendall(
            f"GET /{path} HTTP/1.1\r\nHost: {host}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n"
            f"Sec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n".encode()
        )
        head = b""
        while b"\r\n\r\n" not in head:
            head += self.s.recv(1)
        if b" 101 " not in head.split(b"\r\n")[0]:
            raise RuntimeError(f"no websocket: {head[:80]!r}")
        self.n = 0

    def _send(self, text: str) -> None:
        data = text.encode()
        mask = os.urandom(4)
        n = len(data)
        if n < 126:
            hdr = bytes([0x81, 0x80 | n])
        elif n < 65536:
            hdr = bytes([0x81, 0x80 | 126]) + struct.pack(">H", n)
        else:
            hdr = bytes([0x81, 0x80 | 127]) + struct.pack(">Q", n)
        self.s.sendall(hdr + mask + bytes(b ^ mask[i % 4] for i, b in enumerate(data)))

    def _exact(self, k: int) -> bytes:
        b = b""
        while len(b) < k:
            c = self.s.recv(k - len(b))
            if not c:
                raise RuntimeError("the web view closed the connection")
            b += c
        return b

    def _recv(self) -> str:
        out = b""
        while True:
            b1, b2 = self._exact(2)
            n = b2 & 0x7F
            if n == 126:
                n = struct.unpack(">H", self._exact(2))[0]
            elif n == 127:
                n = struct.unpack(">Q", self._exact(8))[0]
            out += self._exact(n)
            if b1 & 0x80:
                return out.decode()

    def evaluate(self, js: str):
        self.n += 1
        self._send(json.dumps({"id": self.n, "method": "Runtime.evaluate",
                               "params": {"expression": js, "awaitPromise": True, "returnByValue": True}}))
        while True:
            r = json.loads(self._recv())
            if r.get("id") == self.n:
                res = r.get("result", {})
                if "exceptionDetails" in res:
                    raise RuntimeError(json.dumps(res["exceptionDetails"])[:400])
                return res.get("result", {}).get("value")


def app_view() -> Cdp:
    """The app's own web view (not a page's, not another app's): its process's debugging socket, and
    in it the page Tauri serves from `tauri.localhost`."""
    seen: list[str] = []
    for _ in range(90):
        pid = adb("shell", "pidof", PKG, check_rc=False).strip().split(" ")[0]
        socks = [l.split("@")[-1].strip() for l in adb("shell", "cat", "/proc/net/unix", check_rc=False).splitlines()
                 if "webview_devtools_remote" in l]
        mine = [x for x in socks if pid and x.endswith(f"_{pid}")]
        if mine:
            break
        time.sleep(2)
    else:
        raise RuntimeError(f"the app's web view cannot be reached (not a debug build?): sockets {socks}, pid {pid!r}")
    subprocess.run([ADB, "-s", SERIAL, "forward", "tcp:9333", f"localabstract:{mine[0]}"], check=True,
                   capture_output=True)
    for _ in range(90):
        try:
            pages = json.load(urllib.request.urlopen("http://127.0.0.1:9333/json", timeout=5))
            seen = [p.get("url", "") for p in pages]
            page = next(p for p in pages if "tauri.localhost" in p.get("url", "") and "webSocketDebuggerUrl" in p)
            return Cdp(page["webSocketDebuggerUrl"])
        except StopIteration:
            time.sleep(2)
        except Exception:  # noqa: BLE001 - the app is still starting
            time.sleep(2)
    raise RuntimeError(f"the app's page is not in its web view: pages {seen}")


def page_view(part: str) -> Cdp | None:
    """A page view's own web view (a web page formicaria draws), found by its address, through the same
    debugging socket as the app's (`app_view` forwards it)."""
    for _ in range(15):
        try:
            pages = json.load(urllib.request.urlopen("http://127.0.0.1:9333/json", timeout=5))
            page = next(p for p in pages if part in p.get("url", "") and "webSocketDebuggerUrl" in p)
            return Cdp(page["webSocketDebuggerUrl"])
        except Exception:  # noqa: BLE001 - not open yet
            time.sleep(2)
    return None


# **Known trackers and ads, added to a real page** (`privacy-plan-2026-11-19.md` 1.3): five requests
# every list set refuses, made from a Wikipedia page drawn by formicaria; the page view's own count
# (`fm-page-privacy`) must rise by five.
INJECT_JS = r"""
(() => {
  for (const u of ['https://www.google-analytics.com/analytics.js',
                   'https://pagead2.googlesyndication.com/pagead/js/adsbygoogle.js',
                   'https://connect.facebook.net/en_US/fbevents.js',
                   'https://static.hotjar.com/c/hotjar-1.js',
                   'https://securepubads.g.doubleclick.net/tag/js/gpt.js']) {
    const s = document.createElement('script');
    s.src = u + '?fm=' + Date.now();
    document.head.appendChild(s);
  }
  return true;
})()
"""


ROLLER_JS = r"""
(async () => {
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  document.querySelector('button[aria-label="close save media"]')?.click();
  // Two items at least: a new tab's Notes, if fewer are open.
  if (document.querySelectorAll('ul.roller li').length < 2) {
    document.querySelector('button[aria-label="new tab"]')?.click();
    await sleep(800);
    [...document.querySelectorAll('button')].find((b) => b.textContent.trim() === 'Notes')?.click();
    await sleep(1500);
  }
  await sleep(2000); // the app settles after the last case (a page still loading)
  const ul = document.querySelector('ul.roller');
  if (!ul) return { error: 'no roller in the bar' };
  const items = ul.querySelectorAll('li').length;
  // Which item, not its title: two tabs can share one (both YouTube cases open the same page).
  const front = () => {
    const c = ul.querySelector('[aria-current="page"]');
    return c ? (c.closest('li')?.dataset.id || '') + ':' + c.textContent.trim() : undefined;
  };
  const before = front();
  const r = ul.getBoundingClientRect();
  const y = r.top + r.height / 2;
  const at = (t, x) => ul.dispatchEvent(new PointerEvent(t, { bubbles: true, clientX: x, clientY: y, pointerId: 7 }));
  // Towards the first item (a roll to the right goes back), so there is always one to go to.
  const titles = [...ul.querySelectorAll('li')].map((l) => l.dataset.id + ':' + l.textContent.trim().slice(0, 20));
  let tabs = null;
  for (let k = 0; k < localStorage.length; k++) {
    const key = localStorage.key(k);
    if (!/workspace/i.test(key)) continue;
    try {
      const w = JSON.parse(localStorage.getItem(key));
      tabs = (w.panes || []).map((p) => [p.id, p.kind, (p.url || '').slice(0, 50), p.pageTitle]);
    } catch {}
  }
  at('pointerdown', r.left + 40); at('pointermove', r.left + 80); at('pointermove', r.left + 140);
  await sleep(50);
  const live = ul.classList.contains('live');
  at('pointerup', r.left + 140);
  await sleep(800);
  let after = front();
  if (after === before) {
    at('pointerdown', r.right - 40); at('pointermove', r.right - 80); at('pointermove', r.right - 140); at('pointerup', r.right - 140);
    await sleep(800);
    after = front();
  }
  return { items, titles, tabs, width: Math.round(r.width), live, before, after, moved: !!after && after !== before };
})()
"""

# **The first start, as a new person meets it**: the Welcome form, skipped; then the app itself.
FIRST_RUN_JS = r"""
(async () => {
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  for (let i = 0; i < 480; i++) {
    const skip = [...document.querySelectorAll('button')].find((b) => b.textContent.trim() === 'Skip for now');
    if (skip) { skip.click(); await sleep(1500); continue; }
    if (document.querySelector('button[aria-label="new tab"]')) return { ok: true };
    await sleep(500);
  }
  return { ok: false, text: document.body.innerText.slice(0, 400) };
})()
"""

# **Through the app's own screens.** Answers what it saw, for the checks below.
CASE_JS = r"""
(async () => {
  const [page, pick, nth, keep, into] = %s;
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  const until = async (f, secs) => {
    for (let i = 0; i < secs * 4; i++) { let v = null; try { v = f(); } catch {} if (v) return v; await sleep(250); }
    return null;
  };
  const inv = (cmd, args = {}) => window.__TAURI_INTERNALS__.invoke('fm', { cmd, args })
    .then((t) => (typeof t === 'string' ? JSON.parse(t) : t));
  document.querySelector('button[aria-label="close save media"]')?.click();
  document.querySelector('button[aria-label="close the new tab"]')?.click();
  // +, the address, Enter: a page opens inside formicaria.
  const plus = await until(() => document.querySelector('button[aria-label="new tab"]'), 60);
  if (!plus) return { error: 'no + (the app did not start?)', text: document.body.innerText.slice(0, 300) };
  plus.click();
  const field = await until(() => document.querySelector('input[aria-label="a web address or a search"]'), 15);
  if (!field) return { error: 'no address field after +' };
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(field, page);
  field.dispatchEvent(new Event('input', { bubbles: true }));
  await sleep(500);
  field.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
  await sleep(12000);
  // ≡ → Save media.
  const menu = await until(() => document.querySelector('button.menu-btn[aria-label="formicaria\'s options"]'), 20);
  if (!menu) return { error: 'no ≡' };
  menu.click();
  const item = await until(() => [...document.querySelectorAll('[role=menuitem]')]
    .find((b) => b.textContent.trim() === 'Save media'), 10);
  if (!item) return { error: 'no Save media in ≡ (is a page in front?)',
    menu: [...document.querySelectorAll('[role=menuitem]')].map((b) => b.textContent.trim()) };
  item.click();
  const radios = () => [...document.querySelectorAll('input[name=candidate]')];
  await until(() => radios().length, 60);
  const listed = radios().map((i) => i.parentElement.textContent.trim());
  const matching = radios().filter((i) => i.parentElement.textContent.includes(pick));
  const chosen = matching[nth];
  if (!chosen) return { listed, matching: matching.length, error: 'nothing to choose' };
  chosen.click();
  const looked = await until(() => document.querySelector('input[name=what]')
    || (document.querySelector('[role=alert]') || {}).textContent, 300);
  if (!looked || looked.nodeType !== 1)
    return { listed, matching: matching.length, chosen: chosen.parentElement.textContent.trim(),
             error: 'looking: ' + looked, sheet: document.body.innerText.slice(-400) };
  const label = { mp3: 'The sound as MP3', audio: 'The sound, as the site serves it', video: 'The video' }[keep];
  const r = [...document.querySelectorAll('input[name=what]')].find((i) => i.parentElement.textContent.includes(label));
  if (r) r.click();
  await sleep(500);
  // The library before, so only a note this Save made counts (a name like "YouTube video" matched
  // every note by its words, and a failed save passed on an earlier case's note).
  const before = new Set((await inv(into === 'Music' ? 'music_library' : 'video_library')).map((n) => n.id));
  [...document.querySelectorAll('button')].find((b) => b.textContent.trim() === 'Save').click();
  const done = await until(() => {
    const t = document.body.innerText;
    return t.includes('Saved in ' + into) ? 'ok' : (t.match(/Not saved[^\n]*/) || [null])[0];
  }, 900);
  // Not done in time: what the sheet says it is doing (looking, downloading at what share, joining).
  const doing = done ? '' : (document.querySelector('ul.jobs') || document.body).innerText.slice(-300);
  const lib = await inv(into === 'Music' ? 'music_library' : 'video_library');
  const mine = lib.filter((n) => !before.has(n.id));
  // yt-dlp's own words when it failed (the sheet shows formicaria's plain reading of them).
  let raw = [];
  try { raw = JSON.parse(window.__fmMedia.jobs()).filter((j) => j.error).map((j) => j.error.slice(-500)); } catch {}
  const ref = mine[0] && (mine[0].props.audio || mine[0].props.video);
  const [path] = ref ? await inv('asset_paths', { references: [ref] }) : [null];
  document.querySelector('button[aria-label="close save media"]')?.click();
  return { listed, matching: matching.length, chosen: chosen.parentElement.textContent.trim(), done,
           sheet: done === 'ok' ? '' : document.body.innerText.slice(-400), doing, notes: mine.length, ref, path,
           raw: done === 'ok' ? [] : raw.slice(-2) };
})()
"""


# **The phone's protections, on real pages** (`privacy-plan-2026-11-19.md` 1.3): a link with tracking
# parts and a tracker's redirect, each opened as a person does (+, the address, Enter), then known
# trackers and ads added to a real page; what the page view reported back (`fm-page-privacy`,
# `fm-page`) read here.
PRIVACY_JS = r"""
(async () => {
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  const until = async (f, secs) => {
    for (let i = 0; i < secs * 4; i++) { let v = null; try { v = f(); } catch {} if (v) return v; await sleep(250); }
    return null;
  };
  const seen = { blocked: 0, urls: [] };
  addEventListener('fm-page-privacy', (e) => { seen.blocked = Math.max(seen.blocked, e.detail.blocked || 0); });
  addEventListener('fm-page', (e) => { if (e.detail.url) seen.urls.push(e.detail.url); });
  const open = async (address, wait) => {
    document.querySelector('button[aria-label="close save media"]')?.click();
    document.querySelector('button[aria-label="close the new tab"]')?.click();
    const plus = await until(() => document.querySelector('button[aria-label="new tab"]'), 30);
    if (!plus) return false;
    plus.click();
    const field = await until(() => document.querySelector('input[aria-label="a web address or a search"]'), 15);
    if (!field) return false;
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(field, address);
    field.dispatchEvent(new Event('input', { bubbles: true }));
    await sleep(500);
    field.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    await sleep(wait);
    return true;
  };
  const out = {};
  // The first start fetches the lists and builds the engine: how long, and only then the test.
  const t0 = Date.now();
  out.ready = !!(await until(() => window.__fmPage.blockReady && window.__fmPage.blockReady(), 300));
  out.ready_s = Math.round((Date.now() - t0) / 1000);
  try { out.state = JSON.parse(window.__fmPage.blockState()); } catch (e) { out.state = String(e); }
  out.verdicts = ['https://www.google-analytics.com/analytics.js', 'https://pagead2.googlesyndication.com/pagead/js/adsbygoogle.js',
    'https://en.wikipedia.org/static/images/icons/wikipedia.png']
    .map((u) => { try { return window.__fmPage.blockVerdict(u, 'https://en.wikipedia.org/wiki/Formica', 'script'); } catch (e) { return String(e); } });
  seen.urls = [];
  await open('https://en.wikipedia.org/wiki/Ant?utm_source=mail&fbclid=abc123&oldid=1', 8000);
  out.cleaned = seen.urls.slice();
  seen.urls = [];
  await open('https://www.google.com/url?q=https%3A%2F%2Fen.wikipedia.org%2Fwiki%2FFormica&sa=t&usg=x', 8000);
  out.unwrapped = seen.urls.slice();
  // A page with no content security policy of its own, for the known trackers to be added to
  // (Wikipedia's refuses other sites' scripts before any request is made).
  await open('https://example.com/', 6000);
  return out;
})()
"""

VPN_JS = r"""
(async (on) => {
  const inv = (cmd, args = {}) => window.__TAURI_INTERNALS__.invoke('fm', { cmd, args })
    .then((t) => (typeof t === 'string' ? JSON.parse(t) : t));
  const p = await inv('set_privacy', { app_trackers: on });
  window.__fmPage.privacy(JSON.stringify(p));
  for (let i = 0; i < 240; i++) {
    await new Promise((r) => setTimeout(r, 250));
    const st = JSON.parse(window.__fmVpn.state());
    if (st.on === on && (!on || st.ready)) return st;
  }
  return JSON.parse(window.__fmVpn.state());
})(%s)
"""


def resolves(name: str) -> bool:
    """A lookup made on the phone (a shell `ping`, which asks the phone's resolver as apps do)."""
    out = adb("shell", "ping", "-c", "1", "-W", "3", name, check_rc=False, timeout=60)
    return "unknown host" not in out and ("PING" in out or "bytes from" in out)


LOGCAT = WORK / "logcat.txt"


def record_log() -> subprocess.Popen:
    """The phone's log, written as it happens (read afterwards it may be out of reach: a device that
    stopped answering)."""
    adb("logcat", "-c", check_rc=False, timeout=30)
    return subprocess.Popen([ADB, "-s", SERIAL, "logcat", "-b", "main,crash,system", "-v", "time"],
                            stdout=open(LOGCAT, "w"), stderr=subprocess.DEVNULL)


EMU_PROC: list[subprocess.Popen] = []


def emulator_state() -> str:
    """Whether the emulator is still running (and if not, how it ended), with its last words."""
    if not EMU_PROC:
        return "not started"
    rc = EMU_PROC[-1].poll()
    tail = (WORK / "emulator.log").read_text(errors="replace").strip().splitlines()[-3:]
    return ("running" if rc is None else f"ended with {rc}") + "; its log: " + " | ".join(tail)


def phone_log() -> str:
    """What Android said about the app (a crash, a web view that died, memory): its telling lines,
    and the emulator's state."""
    try:
        log = LOGCAT.read_text(errors="replace")
    except OSError:
        return "no log"
    keys = ("FATAL EXCEPTION", "AndroidRuntime", "lowmemorykiller", "Renderer process", "has died",
            "ANR in", "OutOfMemory", "SIGSEGV", "Abort message", "Process " + PKG, PKG + " (pid")
    hits = [l.strip() for l in log.splitlines() if any(k in l for k in keys)]
    return (" | ".join(hits[-8:])[:1000] or "nothing telling in the log") + "; emulator: " + emulator_state()


def main() -> None:
    if not os.access("/dev/kvm", os.R_OK | os.W_OK):
        say("FAIL /dev/kvm is not usable here: the emulator cannot run")
        sys.exit(1)
    WORK.mkdir(parents=True, exist_ok=True)
    site = fixtures()
    srv = serve(site)
    if not APK.exists() or os.environ.get("FM_PHONE_REBUILD", "1") == "1":
        say(f"building the phone app (x86_64, {BUILD})")
        task = "android-apk" if BUILD == "debug" else "android-apk-check"
        subprocess.run(["pixi", "run", "-e", "android", task], cwd=ROOT, check=True)
    apk = APK if BUILD == "debug" else signed(APK)
    emu = boot()
    EMU_PROC.append(emu)
    try:
        if BUILD != "debug":
            adb("root", check_rc=False)
            time.sleep(3)
            adb("wait-for-device", timeout=120)
        adb("reverse", "--remove-all", check_rc=False)
        recorder = record_log()
        # Uninstalled first: a build signed by another key (debug, release, the check's) is refused.
        adb("uninstall", PKG, check_rc=False)
        adb("install", "-t", str(apk), timeout=600)
        # **From a clean app** each run: the emulator keeps an app's data between runs, and tabs left
        # by earlier runs (their clip served on another port, gone) confused the roller's check.
        adb("shell", "pm", "clear", PKG, check_rc=False)
        adb("shell", "am", "start", "-n", f"{PKG}/.MainActivity")
        cdp = app_view()
        first = cdp.evaluate(FIRST_RUN_JS) or {}
        say(f"the first start: {json.dumps(first)[:200]}")
        check("a new person gets past the first start to the app", bool(first.get("ok")), str(first.get("text")))
        say(f"memory after the first start: {app_memory()}")
        # `FM_PHONE_PROBE=<address>`: what the app's yt-dlp lists there, and how formicaria reads it.
        if os.environ.get("FM_PHONE_PROBE"):
            got = cdp.evaluate(r"""
            (async (url) => {
              const s = window.__fmMedia;
              const id = 'dbg-' + Date.now();
              s.probe(id, url);
              for (let i = 0; i < 600; i++) {
                await new Promise((r) => setTimeout(r, 250));
                const j = JSON.parse(s.jobs()).find((x) => x.id === id);
                if (j && (j.state === 'probed' || j.state === 'failed')) {
                  let info = null; try { info = JSON.parse(j.probe || 'null'); } catch {}
                  const fmts = (info && info.formats || []).map((f) => [f.format_id, f.ext, f.vcodec, f.acodec, f.height]);
                  const read = await window.__TAURI_INTERNALS__.invoke('fm', { cmd: 'media_probe_read', args: { json: j.probe || 'null' } });
                  return { state: j.state, error: j.error, version: info && info._version, fmts, read };
                }
              }
              return { error: 'no answer' };
            })(%s)""" % json.dumps(os.environ["FM_PHONE_PROBE"]))
            say(f"probe: {json.dumps(got)[:2000]}")
            return
        # `FM_PHONE_CASES=Ant,clip` runs the cases whose names say so, while working on them.
        only = [k for k in os.environ.get("FM_PHONE_CASES", "").split(",") if k]
        port = srv.server_address[1]
        for name, page, pick, nth, least, keep, into in CASES:
            local = "{port}" in page
            page = page.replace("{port}", str(port))
            if only and not any(k in name for k in only):
                continue
            # The release build refuses plain http (`usesCleartextTraffic`), so this computer's clip
            # is for the debug build; YouTube's own videos come apart the same way, and are joined there.
            if local and BUILD != "debug":
                say(f"skip {name}: plain http, which the release build refuses")
                continue
            got = None
            for attempt in (1, 2):
                try:
                    got = cdp.evaluate(CASE_JS % json.dumps([page, pick, nth, keep, into])) or {}
                    break
                except RuntimeError as e:
                    crashed = EMU_PROC[-1].poll() is not None
                    if crashed and attempt == 1:
                        # **The emulator crashed, not the app** (its software drawing has, on video
                        # pages): started again, and this case tried once more.
                        say(f"{name}: the emulator crashed ({emulator_state()[:120]}); starting it again")
                        EMU_PROC.append(boot())
                        recorder = record_log()
                        adb("shell", "am", "start", "-n", f"{PKG}/.MainActivity", check_rc=False)
                        cdp = app_view()
                        cdp.evaluate(FIRST_RUN_JS)
                        continue
                    who = "the emulator crashed twice (not the app)" if crashed else "the app stays up"
                    check(f"{name}: {who}", False, f"{e}; logcat: {phone_log()}")
                    if not crashed:
                        time.sleep(5)
                        adb("shell", "am", "start", "-n", f"{PKG}/.MainActivity", check_rc=False)
                        cdp = app_view()
                        cdp.evaluate(FIRST_RUN_JS)
                    break
            if got is None:
                continue
            say(f"{name}: {json.dumps(got)[:600]}")
            check(f"{name}: the sheet lists all {least}+", (got.get("matching") or 0) >= least,
                  str(got.get("error") or got.get("listed")))
            check(f"{name}: the sheet says it is saved in {into}", got.get("done") == "ok",
                  f"{got.get('done') or got.get('error')}; yt-dlp said: {got.get('raw')}; the sheet: {got.get('sheet', '')!r}")
            check(f"{name}: its note is in {into}, and it is the one chosen", (got.get("notes") or 0) >= 1,
                  f"chose {got.get('chosen')!r}")
            if got.get("path"):
                head = adb("shell", *AS_APP, "od", "-An", "-c", "-N", "12", got["path"], check_rc=False)
                bare = head.replace(" ", "").replace("\n", "")
                ok = {"mp3": bare.startswith("ID3") or bare.startswith("377"),
                      "video": "ftyp" in bare or bare.startswith("032E"),
                      "audio": len(bare) > 0}[keep]
                check(f"{name}: its file is what was asked ({keep})", ok, repr(head))
                if keep == "video":
                    # Picture **and** sound: a video kept silent is the failure joining exists for.
                    local = WORK / "kept-video"
                    with open(local, "wb") as f:
                        subprocess.run([ADB, "-s", SERIAL, "exec-out", *AS_APP, "cat", got["path"]],
                                       stdout=f, timeout=300, check=False)
                    probe = subprocess.run(["ffprobe", "-v", "error", "-show_entries", "stream=codec_type",
                                            "-of", "csv=p=0", str(local)], capture_output=True, text=True)
                    kinds = probe.stdout.split()
                    check(f"{name}: it has its picture and its sound", "video" in kinds and "audio" in kinds,
                          f"streams: {kinds or probe.stderr.strip()[:200]}")
        # **The roller** (`roller.ts`): the open items on an arc; a roll with the finger brings the
        # next one to the front.
        if not only or "roller" in only:
            got = cdp.evaluate(ROLLER_JS) or {}
            if not got.get("moved"):  # once more: a busy app may have missed the first
                time.sleep(3)
                got = cdp.evaluate(ROLLER_JS) or {}
            say(f"the roller: {json.dumps(got)[:1200]}")
            check("the roller shows the open items", (got.get("items") or 0) >= 2, str(got))
            check("a roll brings the next item to the front", bool(got.get("moved")), str(got))
        say(f"memory after every case: {app_memory()}")
        # **The protections** (`privacy-plan-2026-11-19.md` 1.3), unless only some media cases run.
        if not only or "privacy" in only:
            got = cdp.evaluate(PRIVACY_JS) or {}
            say(f"privacy: {json.dumps(got)[:1500]}")
            check("the pages' lists are read within five minutes of the first start", bool(got.get("ready")),
                  f"{got.get('ready_s')} s")
            cdp.evaluate("window.__fmMax = {}; addEventListener('fm-page-privacy', (e) => {"
                         " window.__fmMax[e.detail.id] = Math.max(window.__fmMax[e.detail.id] || 0, e.detail.blocked || 0); }); true")
            pv = page_view("example.com")
            refused = 0
            if pv:
                pv.evaluate(INJECT_JS)
                time.sleep(8)
                refused = max(cdp.evaluate("Object.values(window.__fmMax)") or [0])
                say(f"  the phone's own counts: {cdp.evaluate('window.__fmPage.blockState()')[:600]}")
                # What the page got for each (an empty answer is a refusal), and whether the engine
                # answers the page's own questions through the bridge.
                seen = pv.evaluate("""(() => ({
                  bridge: typeof __fmPagePrivacy,
                  ran: { googletag: typeof googletag, fbq: typeof fbq, ga: typeof gaplugins },
                  hide: (() => { try { return __fmPagePrivacy.hide(location.href).length; } catch (e) { return String(e); } })(),
                  got: performance.getEntriesByType('resource')
                    .filter((e) => /google-analytics|googlesyndication|fbevents|hotjar|gpt[.]js/.test(e.name))
                    .map((e) => [e.name.slice(8, 50), e.transferSize, e.encodedBodySize, e.responseStatus]),
                }))()""")
                say(f"  the page's view of it: {json.dumps(seen)[:900]}")
            check("five known trackers and ads added to a page are refused", refused >= 5,
                  f"refused {refused}" + ("" if pv else "; the page view could not be reached"))
            cleaned = got.get("cleaned") or []
            check("a link loses its tracking parts", any("wiki/Ant" in u for u in cleaned)
                  and not any("utm_source" in u or "fbclid" in u for u in cleaned), str(cleaned)[:400])
            unwrapped = got.get("unwrapped") or []
            check("a tracker's redirect is skipped (google.com/url)", any("wiki/Formica" in u for u in unwrapped)
                  and not any("google.com/url" in u for u in unwrapped), str(unwrapped)[:400])
            say(f"memory with the pages' lists in use: {app_memory()}")
            # The app blocker, end to end: Android's consent given as a person gives it (`appops`).
            adb("shell", "appops", "set", PKG, "ACTIVATE_VPN", "allow", check_rc=False)
            before = resolves("app-measurement.com")
            st = cdp.evaluate(VPN_JS % "true") or {}
            say(f"app blocker on: {json.dumps(st)[:400]}")
            check("the app blocker starts, its lists read", bool(st.get("on")) and bool(st.get("ready")), str(st)[:300])
            tracker = resolves("app-measurement.com")
            ordinary = resolves("en.wikipedia.org")
            check("an app's tracker lookup is refused, an ordinary one answered",
                  not tracker and ordinary, f"tracker answered: {tracker}; ordinary answered: {ordinary}; before: {before}")
            st = cdp.evaluate(VPN_JS % "false") or {}
            check("off, every lookup is answered again", not st.get("on") and resolves("app-measurement.com"), str(st)[:300])
        # **yt-dlp updates itself** (2026-11-18: it never had: a null class loader, then R8). A fault in
        # the app's code fails; GitHub refusing (60 unsigned requests an hour per address, this
        # computer's) or no network is said, not failed.
        time.sleep(2)
        log = LOGCAT.read_text(errors="replace") if LOGCAT.exists() else ""
        done = [l for l in log.splitlines() if "yt-dlp update:" in l]
        broken = [l for l in log.splitlines() if "yt-dlp not updated:" in l
                  and any(k in l for k in ("NoClassDefFound", "ExceptionInInitializer", "NullPointer", "ClassCast", "NoSuchMethod"))]
        refused = [l for l in log.splitlines() if "yt-dlp not updated:" in l and l not in broken]
        if done:
            say(f"yt-dlp: {done[-1].split('yt-dlp update:')[-1].strip()}")
        elif refused:
            say(f"yt-dlp not updated, not the app's fault: {refused[-1].split('not updated:')[-1].strip()[:200]}")
        check("the phone's yt-dlp can update itself (no fault in the app's code)", not broken,
              broken[-1].split("not updated:")[-1].strip()[:300] if broken else "")
    except Exception as e:  # noqa: BLE001 - said as a failure
        check("the check ran to the end", False, f"{repr(e)[:400]}; logcat: {phone_log()}")
    finally:
        if "recorder" in locals():
            recorder.terminate()
        subprocess.run([ADB, "-s", SERIAL, "emu", "kill"], capture_output=True, timeout=60)
        for e in EMU_PROC:
            try:
                e.wait(timeout=60)
            except Exception:  # noqa: BLE001
                e.kill()
        srv.shutdown()
    say("all good" if not fails else "FAIL — " + "; ".join(fails))
    sys.exit(1 if fails else 0)


if __name__ == "__main__":
    main()
