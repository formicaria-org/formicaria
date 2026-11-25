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
    # **Bandcamp, from its own page** (the owner on 0.59, 2026-11-20: *"YoutubeDLException: ERROR:
    # Unsupported URL: https://bandcamp.com…"*, and the sheet said there was a video): the page
    # declares its sound player as `og:video`, at an address yt-dlp refuses; the sheet must offer
    # "Bandcamp audio" at the page's own address. yt-dlp's own test track, a few seconds long.
    ("Bandcamp: the track, as MP3", "https://youtube-dl.bandcamp.com/track/youtube-dl-test-song",
     "Bandcamp audio", 0, 1, "mp3", "Music"),
    # An album's page is the several-tracks case below (`MULTI_JS`): its tracks are ticked, not chosen.
]

# **Several tracks of an album at once** (the owner, 2026-11-20: *"[Bandcamp:album] … attempting
# impersonation"* on 0.60.1, then *"if more than one track is available I can select the ones I
# want?"*): two ticked, one Save, both kept as MP3 in Music and in one new playlist named after the
# album. yt-dlp's own test album.
ALBUM = ("https://blazo.bandcamp.com/album/jazz-format-mixtape-vol-1", 2, "Jazz Format Mixtape vol.1")


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
    seen: list[str] = []
    for _ in range(15):
        try:
            pages = json.load(urllib.request.urlopen("http://127.0.0.1:9333/json", timeout=5))
            seen = [f"{p.get('type')} {p.get('url', '')[:80]}{'' if 'webSocketDebuggerUrl' in p else ' (attached)'}"
                    for p in pages]
            page = next(p for p in pages if part in p.get("url", "") and "webSocketDebuggerUrl" in p)
            return Cdp(page["webSocketDebuggerUrl"])
        except Exception:  # noqa: BLE001 - not open yet
            time.sleep(2)
    # Said, so a failure names what was there instead (2026-11-20: seen twice after a sixth page).
    say(f"page view {part!r} not found; the debugger lists: {seen}")
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
MULTI_JS = r"""
(async () => {
  const [page, count] = %s;
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  const until = async (f, secs) => {
    for (let i = 0; i < secs * 4; i++) { let v = null; try { v = f(); } catch {} if (v) return v; await sleep(250); }
    return null;
  };
  const inv = (cmd, args = {}) => window.__TAURI_INTERNALS__.invoke('fm', { cmd, args })
    .then((t) => (typeof t === 'string' ? JSON.parse(t) : t));
  document.querySelector('button[aria-label="close save media"]')?.click();
  document.querySelector('button[aria-label="close the new tab"]')?.click();
  const plus = await until(() => document.querySelector('button[aria-label="new tab"]'), 60);
  if (!plus) return { error: 'no +' };
  plus.click();
  const field = await until(() => document.querySelector('input[aria-label="a web address or a search"]'), 15);
  if (!field) return { error: 'no address field after +' };
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(field, page);
  field.dispatchEvent(new Event('input', { bubbles: true }));
  await sleep(500);
  field.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
  await sleep(12000);
  const menu = await until(() => document.querySelector('button.menu-btn[aria-label="formicaria\'s options"]'), 20);
  if (!menu) return { error: 'no ≡' };
  menu.click();
  const item = await until(() => [...document.querySelectorAll('[role=menuitem]')]
    .find((b) => b.textContent.trim() === 'Save media'), 10);
  if (!item) return { error: 'no Save media in ≡' };
  item.click();
  const boxes = () => [...document.querySelectorAll('input[name=track]')];
  await until(() => boxes().length >= count, 60);
  const listed = boxes().map((i) => i.parentElement.textContent.trim());
  if (listed.length < count) return { error: 'tracks to tick: ' + listed.length, sheet: document.body.innerText.slice(-400) };
  if (count === 'all') {
    const all = [...document.querySelectorAll('input[type=checkbox]')].find((i) => /All tracks/.test(i.parentElement.textContent));
    if (!all) return { error: 'no All tracks', listed };
    all.click();
    await sleep(500);
  } else {
    for (const b of boxes().slice(0, count)) { b.click(); await sleep(300); }
  }
  const want = count === 'all' ? boxes().length : count;
  const mp3 = [...document.querySelectorAll('input[name=what]')].find((i) => i.parentElement.textContent.includes('The sound as MP3'));
  if (mp3) mp3.click();
  await sleep(300);
  const sel = document.querySelector('select[aria-label=playlist]');
  if (!sel) return { error: 'no playlist choice', sheet: document.body.innerText.slice(-400) };
  sel.value = '+';
  sel.dispatchEvent(new Event('change', { bubbles: true }));
  await sleep(500);
  const before = new Set((await inv('music_library')).map((n) => n.id));
  const listsBefore = new Set((await inv('music_playlists')).map((p) => p.id));
  const save = [...document.querySelectorAll('button')].find((b) => b.textContent.trim() === `Save ${want} tracks`);
  if (!save) return { error: 'no "Save N tracks"', buttons: [...document.querySelectorAll('button')].map((b) => b.textContent.trim()).slice(-8) };
  save.click();
  const done = await until(() => {
    const t = (document.querySelector('ul.jobs') || {}).innerText || '';
    const bad = t.match(/Not saved[^\n]*/);
    if (bad) return bad[0];
    return (t.match(/Saved in Music/g) || []).length >= want ? 'ok' : null;
  }, count === 'all' ? 2400 : 900);
  const doing = done ? '' : (document.querySelector('ul.jobs') || document.body).innerText.slice(-1500);
  const jobsText = (document.querySelector('ul.jobs') || {}).innerText || '';
  const mine = (await inv('music_library')).filter((n) => !before.has(n.id));
  const lists = (await inv('music_playlists')).filter((p) => !listsBefore.has(p.id));
  document.querySelector('button[aria-label="close save media"]')?.click();
  return { listed: listed.slice(0, 4), want, done, doing, notes: mine.length, titles: mine.map((n) => n.title).slice(0, 30),
           albums: [...new Set(mine.map((n) => n.props.album || '(none)'))],
           saved: (jobsText.match(/Saved in Music/g) || []).length, failed: (jobsText.match(/Not saved[^\n]*/g) || []).slice(0, 5),
           lists: lists.map((p) => ({ title: p.title, songs: (p.songs || []).length })) };
})()
"""

# **A saved song played, as a person plays it** (the owner, 2026-11-20: *"the music view is not
# playing when I click play, are the buttons attached and tested … with real world tests?"* — they
# were not: only a stand-in player had been pressed). + → Music, *play* on a song saved above, then
# what Android's own player (`PlaybackService`, ExoPlayer) says for eight seconds: playing, and how
# far it got.
PLAY_JS = r"""
(async () => {
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  const until = async (f, secs) => {
    for (let i = 0; i < secs * 4; i++) { let v = null; try { v = f(); } catch {} if (v) return v; await sleep(250); }
    return null;
  };
  document.querySelector('button[aria-label="close save media"]')?.click();
  document.querySelector('button[aria-label="close the new tab"]')?.click();
  const seen = { events: 0, playing: false, time: 0, error: '' };
  addEventListener('fm-player', (e) => {
    const d = e.detail || {};
    seen.events++;
    if (d.playing) seen.playing = true;
    seen.time = Math.max(seen.time, d.time || 0);
    if (d.error) seen.error = d.error;
  });
  const plus = await until(() => document.querySelector('button[aria-label="new tab"]'), 30);
  if (!plus) return { error: 'no +' };
  plus.click();
  const music = await until(() => [...document.querySelectorAll('ul[aria-label=features] button')]
    .find((b) => b.textContent.trim() === 'Music'), 15);
  if (!music) return { error: 'no Music in the new tab',
    features: [...document.querySelectorAll('ul[aria-label=features] button')].map((b) => b.textContent.trim()) };
  music.click();
  const play = await until(() => [...document.querySelectorAll('button')]
    .find((b) => (b.getAttribute('aria-label') || '').startsWith('play ')), 30);
  if (!play) return { error: 'no song to play in Music', text: document.body.innerText.slice(0, 300) };
  const pressed = play.getAttribute('aria-label');
  play.click();
  await sleep(8000);
  const said = (document.body.innerText.match(/cannot be played[^\n]*/) || [''])[0];
  const bar = !!document.querySelector('[role=region][aria-label=player]');
  try { window.__fmPlayer.stop(); } catch {}
  return { pressed, bar, said, ...seen };
})()
"""

# **A note holding audio, played and put in a playlist, through Music** (the owner, 2026-11-21: an
# album saved *Into the note* played nowhere; *"create a playlist by taking songs from anywhere"*). A
# song saved above has its file put into an ordinary note (set up through the app's own command);
# then, as a person: + → Music → Playlists → *In your notes* → Play, and Add to playlist → a new one.
NOTE_JS = r"""
(async () => {
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  const until = async (f, secs) => {
    for (let i = 0; i < secs * 4; i++) { let v = null; try { v = f(); } catch {} if (v) return v; await sleep(250); }
    return null;
  };
  const inv = (cmd, args = {}) => window.__TAURI_INTERNALS__.invoke('fm', { cmd, args })
    .then((t) => (typeof t === 'string' ? JSON.parse(t) : t));
  const song = (await inv('music_library'))[0];
  const ref = song && song.props.audio;
  if (!ref) return { error: 'no song saved above to take a file from' };
  await inv('create_notes', { notes: [{ title: 'A note with a recording', props: {},
    body: `Heard today.\n\n![A recording](asset:${ref.replace('sha256:', 'sha256-')})\n` }] });
  document.querySelector('button[aria-label="close save media"]')?.click();
  document.querySelector('button[aria-label="close the new tab"]')?.click();
  const seen = { playing: false, time: 0, error: '' };
  addEventListener('fm-player', (e) => {
    const d = e.detail || {};
    if (d.playing) seen.playing = true;
    seen.time = Math.max(seen.time, d.time || 0);
    if (d.error) seen.error = d.error;
  });
  const plus = await until(() => document.querySelector('button[aria-label="new tab"]'), 30);
  if (!plus) return { error: 'no +' };
  plus.click();
  const music = await until(() => [...document.querySelectorAll('ul[aria-label=features] button')]
    .find((b) => b.textContent.trim() === 'Music'), 15);
  if (!music) return { error: 'no Music in the new tab' };
  music.click();
  const tab = await until(() => [...document.querySelectorAll('[role=tab]')].find((b) => b.textContent.trim() === 'Playlists'), 30);
  if (!tab) return { error: 'no Playlists tab' };
  tab.click();
  const list = await until(() => document.querySelector('ul[aria-label="notes with audio"]'), 30);
  if (!list) return { error: 'no "In your notes"', text: document.body.innerText.slice(0, 400) };
  const play = list.querySelector('button[aria-label="play A note with a recording"]');
  if (!play) return { error: 'the note is not listed', listed: list.innerText.slice(0, 300) };
  play.click();
  await sleep(8000);
  try { window.__fmPlayer.stop(); } catch {}
  const add = list.querySelector('button[aria-label="add A note with a recording to a playlist"]');
  add.click();
  const sheet = await until(() => document.querySelector('[role=dialog][aria-label="A note with a recording"]'), 15);
  if (!sheet) return { ...seen, error: 'no sheet after Add to playlist' };
  [...sheet.querySelectorAll('button')].find((b) => b.textContent.trim() === 'Add to playlist…')?.click();
  const name = await until(() => sheet.querySelector('input[aria-label="name of the new playlist"]'), 10);
  if (!name) return { ...seen, error: 'no name field' };
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(name, 'Heard in notes');
  name.dispatchEvent(new Event('input', { bubbles: true }));
  [...sheet.querySelectorAll('button')].find((b) => b.textContent.trim() === 'Make it')?.click();
  await sleep(3000);
  const lists = (await inv('music_playlists')).filter((p) => p.title === 'Heard in notes');
  return { ...seen, lists: lists.map((p) => p.songs.length) };
})()
"""

# **An album keeps saving through a closed app** (2026-11-21: an album saved on a phone that went to
# the background kept its first track only). Four tracks ticked and saved; once the first is in, the
# app is stopped as Android stops one, and started again: all four must arrive, once each.
START_ALBUM_JS = r"""
(async () => {
  const [page, count, from] = %s;
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  const until = async (f, secs) => {
    for (let i = 0; i < secs * 4; i++) { let v = null; try { v = f(); } catch {} if (v) return v; await sleep(250); }
    return null;
  };
  document.querySelector('button[aria-label="close save media"]')?.click();
  document.querySelector('button[aria-label="close the new tab"]')?.click();
  const plus = await until(() => document.querySelector('button[aria-label="new tab"]'), 60);
  if (!plus) return { error: 'no +' };
  plus.click();
  const field = await until(() => document.querySelector('input[aria-label="a web address or a search"]'), 15);
  if (!field) return { error: 'no address field' };
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(field, page);
  field.dispatchEvent(new Event('input', { bubbles: true }));
  await sleep(500);
  field.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
  await sleep(12000);
  const menu = await until(() => document.querySelector('button.menu-btn[aria-label="formicaria\'s options"]'), 20);
  if (!menu) return { error: 'no ≡' };
  menu.click();
  const item = await until(() => [...document.querySelectorAll('[role=menuitem]')].find((b) => b.textContent.trim() === 'Save media'), 10);
  if (!item) return { error: 'no Save media' };
  item.click();
  const boxes = () => [...document.querySelectorAll('input[name=track]')];
  await until(() => boxes().length >= from + count, 60);
  for (const b of boxes().slice(from, from + count)) { b.click(); await sleep(300); }
  const mp3 = [...document.querySelectorAll('input[name=what]')].find((i) => i.parentElement.textContent.includes('The sound as MP3'));
  if (mp3) mp3.click();
  await sleep(300);
  const sel = document.querySelector('select[aria-label=playlist]');
  sel.value = '+';
  sel.dispatchEvent(new Event('change', { bubbles: true }));
  await sleep(500);
  const save = [...document.querySelectorAll('button')].find((b) => b.textContent.trim() === `Save ${count} tracks`);
  if (!save) return { error: 'no "Save N tracks"' };
  save.click();
  return { started: true };
})()
"""
COUNT_JS = r"""
(async () => {
  const inv = (cmd, args = {}) => window.__TAURI_INTERNALS__.invoke('fm', { cmd, args })
    .then((t) => (typeof t === 'string' ? JSON.parse(t) : t));
  const lib = await inv('music_library');
  const lists = await inv('music_playlists');
  return { songs: lib.length, titles: lib.map((n) => n.title),
           lists: lists.map((p) => ({ title: p.title, songs: (p.songs || []).length })),
           queue: (() => { try { return JSON.parse(localStorage.getItem('fm-media-queue') || '[]').length; } catch { return -1; } })() };
})()
"""

# **The music player, every button, songs one after another** (the owner, 2026-11-21): three songs
# saved above, at least half a minute each, put in a playlist; Music → Playlists → it → Play, as a person
# does; then `ci/music-tour.js`, the same walk formicaria's browser gets in `usability-check`.
MUSIC_SETUP_JS = r"""
(async () => {
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  const until = async (f, secs) => {
    for (let i = 0; i < secs * 4; i++) { let v = null; try { v = f(); } catch {} if (v) return v; await sleep(250); }
    return null;
  };
  const inv = (cmd, args = {}) => window.__TAURI_INTERNALS__.invoke('fm', { cmd, args })
    .then((t) => (typeof t === 'string' ? JSON.parse(t) : t));
  const lib = await inv('music_library');
  const long = lib.filter((n) => (n.props.duration_s || 0) >= 30);
  const three = (long.length >= 3 ? long : lib).slice(0, 3);
  if (three.length < 3) return { error: `only ${three.length} songs saved above` };
  const body = three.map((n, i) => `${i + 1}. [${n.title}](note:${n.id})`).join('\n') + '\n';
  await inv('create_notes', { notes: [{ title: 'Three songs', tags: ['playlist'], body, props: {} }] });
  document.querySelector('button[aria-label="close save media"]')?.click();
  document.querySelector('button[aria-label="close the new tab"]')?.click();
  const plus = await until(() => document.querySelector('button[aria-label="new tab"]'), 30);
  if (!plus) return { error: 'no +' };
  plus.click();
  const music = await until(() => [...document.querySelectorAll('ul[aria-label=features] button')].find((b) => b.textContent.trim() === 'Music'), 15);
  if (!music) return { error: 'no Music' };
  music.click();
  // **Within the Music screen just opened** (2026-11-21): an earlier case leaves another Music open,
  // and every list has its own Play — the first in the page played that one's songs instead.
  const box = () => [...document.querySelectorAll('.music-box')].filter((b) => b.offsetParent !== null).pop();
  const tab = await until(() => box() && [...box().querySelectorAll('[role=tab]')].find((b) => b.textContent.trim() === 'Playlists'), 30);
  if (!tab) return { error: 'no Playlists tab' };
  tab.click();
  const list = await until(() => box() && box().querySelector('button[aria-label="playlist Three songs"]'), 30);
  if (!list) return { error: 'the playlist is not listed' };
  list.click();
  // Its own Play, once it is drawn: right after the press the Playlists tab is still on screen, with
  // *In your notes* and its Play buttons, which play a note's recording instead (2026-11-21).
  const play = await until(() => box() && box().querySelector('ul[aria-label="songs of Three songs"]')
    && box().querySelector('.bulk button.go'), 15);
  if (!play) return { error: 'no Play in the playlist' };
  const rows = box().querySelectorAll('li').length;
  const lists = document.querySelectorAll('button[aria-label="playlist Three songs"]').length;
  const disabled = play.disabled;
  play.click();
  await sleep(3000);
  const deckNow = [...document.querySelectorAll('section[aria-label="music player"] .sr')].map((e) => e.textContent.trim());
  return { titles: three.map((n) => n.title), seen: { rows, lists, disabled, deckNow, boxes: document.querySelectorAll('.music-box').length } };
})()
"""

# **Pages that would not open on a phone** (the owner, 2026-11-22: some papers and some researchers'
# homepages): a site with no https at all must offer *Open it anyway* within
# seconds and then open over plain http (the release build forbade it); a PDF must reach the phone's
# PDF app (Android's page view cannot show one).
OPEN_JS = r"""
(async () => {
  const page = %s;
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  const until = async (f, secs) => {
    for (let i = 0; i < secs * 4; i++) { let v = null; try { v = f(); } catch {} if (v) return v; await sleep(250); }
    return null;
  };
  document.querySelector('button[aria-label="close save media"]')?.click();
  document.querySelector('button[aria-label="close the new tab"]')?.click();
  const plus = await until(() => document.querySelector('button[aria-label="new tab"]'), 30);
  if (!plus) return { error: 'no +' };
  plus.click();
  const field = await until(() => document.querySelector('input[aria-label="a web address or a search"]'), 15);
  if (!field) return { error: 'no address field' };
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(field, page);
  field.dispatchEvent(new Event('input', { bubbles: true }));
  await sleep(500);
  field.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
  return { opened: page };
})()
"""


def pages_text() -> list:
    """Every page view the debugger lists, with its address and the start of its text."""
    out = []
    try:
        pages = json.load(urllib.request.urlopen("http://127.0.0.1:9333/json", timeout=5))
    except Exception:  # noqa: BLE001
        return out
    for p in pages:
        if "webSocketDebuggerUrl" not in p:
            continue
        try:
            c = Cdp(p["webSocketDebuggerUrl"])
            t = c.evaluate("(document.title || '') + ' | ' + (document.body ? document.body.innerText.slice(0, 300) : '')") or ""
            out.append((p.get("url", ""), t, c))
        except Exception:  # noqa: BLE001
            pass
    return out


# **A case's page closed once it is done**, as a person closes a tab: the app holds at most eight
# panes (`MAX_PANES`, `ui/src/lib/panes.ts`) and refuses a ninth, which the privacy check's own pages
# then were (2026-11-20, after the Bandcamp case made eight). Only a web page in front is closed —
# the one ≡ offers *Save media* for — never one of the app's own views.
CLOSE_JS = r"""
(async () => {
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  document.querySelector('button[aria-label="close save media"]')?.click();
  await sleep(500);
  const menu = document.querySelector('button.menu-btn[aria-label="formicaria\'s options"]');
  if (!menu) return 'no ≡';
  menu.click();
  await sleep(500);
  const page = [...document.querySelectorAll('[role=menuitem]')].some((b) => b.textContent.trim() === 'Save media');
  document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
  if (document.querySelector('[role=menuitem]')) menu.click();
  await sleep(300);
  if (!page) return 'no page in front';
  const x = document.querySelector('li.tab.on button.close');
  if (!x) return 'no close button';
  const said = x.getAttribute('aria-label');
  x.click();
  await sleep(800);
  return said;
})()
"""

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
DIARY_JS = r"""
(async () => {
  const t = await window.__TAURI_INTERNALS__.invoke('fm', { cmd: 'diary_status', args: {} });
  return typeof t === 'string' ? JSON.parse(t) : t;
})()
"""


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


LAYOUT_GUARD = "(function () {\n" + (Path(__file__).parent / "layout-guard.js").read_text() + "\n})()"

HOME_JS = r"""
(async () => {
  const wait = (ms) => new Promise((r) => setTimeout(r, ms));
  for (let i = 0; i < 40 && !document.querySelector('.home'); i++) {
    const b = [...document.querySelectorAll('button, a')].find(
      (x) => (x.getAttribute('aria-label') || x.textContent || '').trim() === 'Home');
    if (b) b.click();
    await wait(500);
  }
  return !!document.querySelector('.home');
})()"""


def guard(cdp, where: str) -> None:
    """The layout guard (`ci/layout-guard.js`) on the phone's screen as it is."""
    problems = cdp.evaluate(LAYOUT_GUARD) or []
    check(f"{where}: every control is on the screen and uncovered", not problems, "; ".join(problems[:6]))


def home_on_the_phone(cdp) -> None:
    """Home, Edit Home and the journal on the phone, with the keyboard up (2026-11-23)."""
    cdp.evaluate("""window.__TAURI_INTERNALS__.invoke('fm', { cmd: 'create_notes', args: { notes: [
        { title: 'Order the reagents', body: '', tags: [], start: '', props: { status: 'todo' } }] } })""")
    shown = cdp.evaluate(HOME_JS)
    check("Home opens on the phone", bool(shown))
    if not shown:
        return
    time.sleep(3)
    guard(cdp, "Home on the phone")
    cdp.evaluate("document.querySelector('button.customize').click()")
    time.sleep(1.5)
    guard(cdp, "Edit Home on the phone")
    # A card added on the phone is on the phone's Home.
    cdp.evaluate("""(() => { const s = document.querySelector('select[aria-label="card to add"]');
        s.value = 'heat:notes'; s.dispatchEvent(new Event('change', { bubbles: true })); })()""")
    # The choice enables the button when the screen next draws: pressed after it, as a person would.
    time.sleep(1)
    cdp.evaluate("""[...document.querySelectorAll('button')].find((b) => b.textContent.trim() === 'Add to Home').click()""")
    time.sleep(3)
    added = cdp.evaluate("!!document.querySelector('[role=img][aria-label^=\"Notes, day by day\"]')")
    if not added:
        say("adding on the phone: " + json.dumps(cdp.evaluate("""(() => ({ err: (document.querySelector('.home .error') || {}).textContent || '',
            cards: [...document.querySelectorAll('[aria-label="home cards shown"] > li h3')].map((h) => h.textContent.trim()),
            listed: [...document.querySelectorAll('[aria-label="home cards"] .name')].map((n) => n.textContent.trim()),
            adding: (document.querySelector('select[aria-label="card to add"]') || {}).value,
            narrow: document.querySelector('.home').classList.contains('narrow') }))()""")))
    check("a card added on the phone is shown on the phone's Home", bool(added))
    cdp.evaluate("document.querySelector('button.customize').click()")
    time.sleep(1.5)
    # The journal, tapped and typed into for real: the keyboard comes up.
    box = cdp.evaluate("""(() => { const c = document.querySelector('section[aria-label^="journal, "] .cm-content');
        if (!c) return null; c.scrollIntoView({ block: 'center' }); const r = c.getBoundingClientRect();
        return { x: r.left + Math.min(40, r.width / 2), y: r.top + Math.min(20, r.height / 2), dpr: devicePixelRatio,
                 h: innerHeight }; })()""")
    check("the journal entry is on the phone's Home", bool(box), str(box))
    if not box:
        return
    time.sleep(1)
    box = cdp.evaluate("""(() => { const r = document.querySelector('section[aria-label^="journal, "] .cm-content').getBoundingClientRect();
        return { x: r.left + Math.min(40, r.width / 2), y: r.top + Math.min(20, r.height / 2), dpr: devicePixelRatio, h: innerHeight }; })()""")
    adb("shell", "input", "tap", str(int(box["x"] * box["dpr"])), str(int(box["y"] * box["dpr"])), check_rc=False)
    time.sleep(2)
    adb("shell", "input", "text", "Fed%sthe%scolony%sat%snoon", check_rc=False)
    time.sleep(1)
    adb("shell", "input", "keyevent", "66", check_rc=False)  # Enter: a second line
    adb("shell", "input", "text", "and%sagain%sin%sthe%sevening", check_rc=False)
    time.sleep(2.5)
    state = cdp.evaluate("""(() => { const h = document.querySelector('.home'); const j = document.querySelector('.capture');
        const vv = window.visualViewport;
        const bar = document.querySelector('nav.tab-bar'); const b = bar ? bar.getBoundingClientRect() : null;
        return { writing: !!h && h.classList.contains('writing'), jot: j ? getComputedStyle(j).visibility : 'none',
                 typing: document.documentElement.hasAttribute('data-typing'),
                 bar: bar ? { shown: getComputedStyle(bar).visibility, h: Math.round(b.height) } : null,
                 keyboard: vv ? Math.round(innerHeight - vv.height) : null, view: vv ? Math.round(vv.height) : innerHeight,
                 typed: (document.querySelector('section[aria-label^="journal, "] .cm-content') || {}).textContent || '' }; })()""") or {}
    say(f"the journal, typed into on the phone: {json.dumps(state)[:400]}")
    shot = WORK / "phone-home-keyboard.png"
    try:
        shot.write_bytes(subprocess.run([ADB, "-s", SERIAL, "exec-out", "screencap", "-p"],
                                        capture_output=True, timeout=60).stdout)
        say(f"the screen with the keyboard up: {shot}")
    except Exception as e:  # noqa: BLE001
        say(f"no screenshot: {e}")
    check("what is typed into the journal on the phone is there", "colony at noon" in state.get("typed", ""),
          str(state)[:200])
    check("typing in the journal on the phone gives it the screen; the jot box gives way",
          state.get("writing") is True and state.get("jot") in ("hidden", "none"), str(state))
    # **The bottom bar steps aside while typing** (2026-11-24): it sat between the journal and the
    # keyboard. Gone means out of reach and taking no room, not merely see-through.
    bar = state.get("bar") or {}
    check("typing in the journal on the phone moves the bottom bar aside",
          state.get("typing") is True and bar.get("shown") == "hidden" and bar.get("h") == 0, str(state)[:300])
    guard(cdp, "the journal, typed into on the phone with the keyboard up")
    adb("shell", "input", "keyevent", "4", check_rc=False)  # Back: the keyboard goes
    time.sleep(1.5)
    # Putting the keyboard away brings the bar back, though the journal still has the focus.
    back = cdp.evaluate("""(() => { const bar = document.querySelector('nav.tab-bar'); const b = bar.getBoundingClientRect();
        return { typing: document.documentElement.hasAttribute('data-typing'), shown: getComputedStyle(bar).visibility,
                 h: Math.round(b.height), bottom: Math.round(b.bottom), view: innerHeight }; })()""") or {}
    check("the bottom bar comes back when the keyboard goes",
          back.get("typing") is False and back.get("shown") == "visible" and back.get("h", 0) > 30
          and back.get("bottom", 1e9) <= back.get("view", 0) + 1, str(back))
    cdp.evaluate("document.activeElement && document.activeElement.blur()")
    time.sleep(1)


def home_rearranged_on_the_phone(cdp) -> None:
    """**Home rearranged on the phone** (2026-11-25): a card's ⠿ held and dragged with a finger goes
    before the card it is let go over; a swipe that starts on it still scrolls (it waits for a hold)."""
    shown = cdp.evaluate(HOME_JS)
    if not shown:
        check("Home opens on the phone, to rearrange", False)
        return
    time.sleep(2)
    order = lambda: cdp.evaluate("[...document.querySelectorAll('ol.grid > li.card')].map((li) => li.dataset.ref)") or []  # noqa: E731
    before = order()
    if len(before) < 2:
        check("the phone's Home has two cards to rearrange", False, str(before))
        return
    pts = cdp.evaluate("""(() => { const cards = [...document.querySelectorAll('ol.grid > li.card')];
        const a = cards[0], b = cards[1]; a.scrollIntoView({ block: 'start' });
        const g = b.querySelector('.move-grip').getBoundingClientRect(); const r = a.getBoundingClientRect();
        return { gx: g.left + g.width / 2, gy: g.top + g.height / 2, tx: r.left + r.width / 2, ty: r.top + Math.min(40, r.height / 3),
                 dpr: devicePixelRatio, view: innerHeight }; })()""")
    say(f"rearranging on the phone: {json.dumps(pts)}")
    if not pts or pts["gy"] > pts["view"] - 10:
        check("the second card's grip is on the phone's screen", False, str(pts))
        return
    d = pts["dpr"]
    adb("shell", "input", "touchscreen", "draganddrop", str(int(pts["gx"] * d)), str(int(pts["gy"] * d)),
        str(int(pts["tx"] * d)), str(int(pts["ty"] * d)), "1500", check_rc=False)
    time.sleep(3)
    after = order()
    check("a card held by its grip and dragged on the phone goes before the card it is let go over",
          len(after) >= 2 and after[0] == before[1] and after[1] == before[0], f"{before} → {after}")
    guard(cdp, "Home on the phone, rearranged")

def new_tab_on_the_phone(cdp) -> None:
    """The ＋ sheet on the phone (2026-11-25; the owner: its last row of features lay under Android's
    back and home buttons). Tapped for real, so the field it focuses brings the keyboard up; then
    scrolled to its end, where the last row must stand above the navigation bar."""
    box = cdp.evaluate("""(() => { const b = document.querySelector('nav.tab-bar button.plus');
        if (!b) return null; const r = b.getBoundingClientRect();
        return { x: r.left + r.width / 2, y: r.top + r.height / 2, dpr: devicePixelRatio }; })()""")
    check("the phone's bar has its ＋", bool(box))
    if not box:
        return
    adb("shell", "input", "tap", str(int(box["x"] * box["dpr"])), str(int(box["y"] * box["dpr"])), check_rc=False)
    time.sleep(2.5)
    check("＋ opens the new-tab sheet on the phone",
          bool(cdp.evaluate("!!document.querySelector('.new-tab-sheet')")))
    guard(cdp, "the ＋ sheet on the phone, as it opens")
    cdp.evaluate("document.activeElement && document.activeElement.blur()")
    time.sleep(1.5)
    end = cdp.evaluate("""(() => { const s = document.querySelector('.new-tab-sheet'); if (!s) return null;
        s.scrollTop = s.scrollHeight; const last = [...s.querySelectorAll('.features button')].pop();
        const probe = (v) => { const p = document.createElement('div');
          p.style.cssText = `position:fixed;visibility:hidden;top:0;height:var(${v}, 0px)`;
          document.body.appendChild(p); const h = p.getBoundingClientRect().height; p.remove(); return h; };
        const r = last.getBoundingClientRect();
        return { last: last.textContent.trim(), bottom: Math.round(r.bottom), view: innerHeight,
                 nav: Math.round(probe('--safe-bottom')),
                 kb: parseFloat(document.documentElement.style.getPropertyValue('--kb')) || 0 }; })()""") or {}
    shot = WORK / "phone-new-tab.png"
    try:
        shot.write_bytes(subprocess.run([ADB, "-s", SERIAL, "exec-out", "screencap", "-p"],
                                        capture_output=True, timeout=60).stdout)
        say(f"the ＋ sheet scrolled to its end: {shot}")
    except Exception as e:  # noqa: BLE001
        say(f"no screenshot: {e}")
    check("the ＋ sheet's last row stands above the navigation bar",
          bool(end) and end["bottom"] <= end["view"] - end["kb"] - end["nav"] + 1, json.dumps(end))
    guard(cdp, "the ＋ sheet on the phone, scrolled to its end")
    cdp.evaluate("document.querySelector('.new-tab-close') && document.querySelector('.new-tab-close').click()")
    time.sleep(1)

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
        # The page may reload under the script while it waits (its answer is then empty, 2026-11-21:
        # every case after failed on the welcome screen): taken again, on the page as it is now.
        first = {}
        for attempt in range(3):
            try:
                first = cdp.evaluate(FIRST_RUN_JS) or {}
            except RuntimeError as e:
                first = {"text": str(e)[:300]}
            if first.get("ok"):
                break
            say(f"the first start, try {attempt + 1}: {json.dumps(first)[:200]}; again")
            time.sleep(3)
            cdp = app_view()
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
        # **Home on the phone, as a person uses it** (2026-11-23; the owner: Edit Home's ✕ off the
        # screen, a board edited on the phone not on it, and typing into the journal showing Home's
        # top, a floating bar and a sliver of the entry). With the keyboard really up.
        if not only or "home" in only:
            home_on_the_phone(cdp)
            home_rearranged_on_the_phone(cdp)
        if not only or "plus" in only:
            new_tab_on_the_phone(cdp)
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
            # **A refusal from the site, once** (2026-11-21: YouTube's video servers answered 403 in
            # two runs of four, the same build passing between): the case is tried once more. A
            # breakage that lasts fails the second time as well.
            if "changed how it serves media" in str(got.get("done") or ""):
                say(f"{name}: the site refused ({got.get('done')}); once more")
                try:
                    cdp.evaluate(CLOSE_JS)
                    time.sleep(10)
                    got = cdp.evaluate(CASE_JS % json.dumps([page, pick, nth, keep, into])) or {}
                except RuntimeError as e:
                    got = {"error": str(e)[:300]}
            say(f"{name}: {json.dumps(got)[:600]}")
            try:
                say(f"{name}: its page {cdp.evaluate(CLOSE_JS)}")
            except RuntimeError as e:
                say(f"{name}: its page could not be closed: {e}")
            check(f"{name}: the sheet lists all {least}+", (got.get("matching") or 0) >= least,
                  str(got.get("error") or got.get("listed")))
            if "Bandcamp" in name:
                # The page shows no video, and the sheet must not say there is one.
                check(f"{name}: nothing on it is said to be a video",
                      not any("video" in str(x).lower() for x in got.get("listed") or []),
                      str(got.get("listed")))
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
        if "album-all" in only:
            name = "Bandcamp: a whole album, All tracks"
            got = cdp.evaluate(MULTI_JS % json.dumps([ALBUM[0], "all"])) or {}
            say(f"{name}: {json.dumps(got)[:3000]}")
            check(f"{name}: every track is saved in Music", got.get("done") == "ok" and got.get("notes") == got.get("want"),
                  f"{got.get('error') or got.get('done')}; notes {got.get('notes')} of {got.get('want')}; failed {got.get('failed')}")
            check(f"{name}: all are in one playlist named after the album",
                  got.get("lists") == [{"title": ALBUM[2], "songs": got.get("want")}], str(got.get("lists")))
            check(f"{name}: each is filed under the album", got.get("albums") == [ALBUM[2]], str(got.get("albums")))
        if not only or any("Bandcamp" in k or "several" in k for k in only):
            name = "Bandcamp: two tracks of an album, ticked"
            got = cdp.evaluate(MULTI_JS % json.dumps(list(ALBUM[:2]))) or {}
            say(f"{name}: {json.dumps(got)[:900]}")
            check(f"{name}: both are saved in Music", got.get("done") == "ok" and got.get("notes") == ALBUM[1],
                  f"{got.get('error') or got.get('done')}; notes {got.get('notes')}; {got.get('doing', '')!r}")
            check(f"{name}: both are in one new playlist named after the album",
                  got.get("lists") == [{"title": ALBUM[2], "songs": ALBUM[1]}], str(got.get("lists")))
            try:
                say(f"{name}: its page {cdp.evaluate(CLOSE_JS)}")
            except RuntimeError as e:
                say(f"{name}: its page could not be closed: {e}")
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
        # After the protections: Music in front would leave their test page paused (2026-11-21).
        if not only or any("play" in k or "note" in k for k in only):
            got = cdp.evaluate(NOTE_JS) or {}
            say(f"a note holding audio: {json.dumps(got)[:900]}")
            check("a note's recording plays from Music (Android's player says so)",
                  bool(got.get("playing")) and not got.get("error"), f"{got.get('error') or got}")
            check("its recording goes into a new playlist as a song", got.get("lists") == [1], str(got))
        if not only or any("play" in k for k in only):
            got = cdp.evaluate(PLAY_JS) or {}
            say(f"playing a saved song: {json.dumps(got)[:900]}")
            check("a song saved here plays when its play is pressed (Android's player says so)",
                  bool(got.get("playing")) and not got.get("error") and not got.get("said"),
                  f"{got.get('error') or got.get('said') or got}")
            check("its position moves on while it plays", (got.get("time") or 0) >= 2,
                  f"reached {got.get('time')} s; {got.get('events')} reports from the player")
        # Last of the music: it leaves a playlist open on the Music screen.
        if not only or "music" in only:
            setup = cdp.evaluate(MUSIC_SETUP_JS) or {}
            say(f"the music player: {json.dumps(setup)[:400]}")
            if setup.get("titles"):
                tour = (ROOT / "ci" / "music-tour.js").read_text()
                got = cdp.evaluate(f"({tour})({json.dumps(setup['titles'])})") or {}
                for step, st in (got.get("steps") or {}).items():
                    check(f"music: {step}", st.get("ok"), st.get("said", ""))
                if not got.get("steps"):
                    check("music: the player could be gone through", False, str(got)[:300])
                cdp.evaluate("try { window.__fmPlayer.stop(); } catch (e) {} true")
            else:
                check("music: a playlist of three songs plays", False, str(setup)[:300])
        # **The usage diary** (`fm_app::diary`, 2026-11-20): on by default, and this run in it — its
        # start and its commands — read through the app as Settings reads it.
        got = cdp.evaluate(DIARY_JS) or {}
        say(f"usage diary: {json.dumps(got)[:300]}")
        check("the phone keeps its usage diary", bool(got.get("on")) and (got.get("bytes") or 0) > 0, str(got)[:300])
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
        if not only or "web" in only:
            # **A site with no https at all opens over http by itself** (2026-11-25, *secure first, never
            # a wall*): tried over https first, then the page, within seconds; no page asking first,
            # which is only for *Always https*.
            started = time.time()
            say(f"opening an http-only homepage: {cdp.evaluate(OPEN_JS % json.dumps('http://piketty.pse.ens.fr/en/'))}")
            shown = asked = None
            while time.time() - started < 25 and not shown:
                time.sleep(1)
                texts = pages_text()
                asked = asked or next(((u, t) for u, t, _ in texts if "No secure connection" in t), None)
                shown = next(((u, t) for u, t, _ in texts if "piketty.pse.ens.fr" in u and u.startswith("http://")
                              and "No secure" not in t and len(t) > 20), None)
            waited = round(time.time() - started)
            check("a site with no https opens over plain http by itself, within seconds",
                  bool(shown) and not asked and waited <= 15,
                  f"after {waited} s, asked first {bool(asked)}: {[(u[:60], t[:60]) for u, t, _ in pages_text()]}")
            cdp.evaluate(CLOSE_JS)
            # A PDF: the phone's PDF app is offered.
            adb("logcat", "-c", check_rc=False)
            say(f"opening a PDF: {cdp.evaluate(OPEN_JS % json.dumps('https://arxiv.org/pdf/2401.00001'))}")
            chooser = ""
            for _ in range(40):
                time.sleep(1)
                # Any sign the PDF reached a viewer: Android's chooser, a PDF app's window, or our
                # file's content link handed over.
                log = adb("logcat", "-d", check_rc=False, timeout=60)
                chooser = next((l for l in log.splitlines() if any(k in l for k in (
                    "action.CHOOSER", "ChooserActivity", "ResolverActivity", "PdfViewer",
                    "fileprovider/vault/web-pdf"))), "")
                if chooser:
                    break
            check("a PDF is handed to the phone's PDF app (Android's chooser opens)", bool(chooser),
                  chooser[:200] or f"no chooser in the log; pages: {[(u[:80], t[:60]) for u, t, _ in pages_text()]}; "
                  f"focus: {adb('shell', 'dumpsys', 'window', check_rc=False, timeout=60).count('ChooserActivity')}")
            adb("shell", "input", "keyevent", "4", check_rc=False)
            time.sleep(1)
            cdp.evaluate(CLOSE_JS)
        if not only or "resume" in only:
            name = "Bandcamp: an album, the app stopped half way"
            base = (cdp.evaluate(COUNT_JS) or {})
            got = cdp.evaluate(START_ALBUM_JS % json.dumps([ALBUM[0], 4, 10])) or {}
            say(f"{name}: started {json.dumps(got)}; before {base.get('songs')} songs")
            first = None
            for _ in range(120):
                now = cdp.evaluate(COUNT_JS) or {}
                if (now.get("songs") or 0) > (base.get("songs") or 0):
                    first = now
                    break
                time.sleep(2)
            say(f"{name}: the first in: {json.dumps(first)[:400]}")
            # Stopped as Android stops an app it needs the memory of, and opened again.
            adb("shell", "am", "force-stop", PKG, check_rc=False)
            time.sleep(3)
            adb("shell", "am", "start", "-n", f"{PKG}/.MainActivity", check_rc=False)
            cdp = app_view()
            after = {}
            for _ in range(300):
                try:
                    after = cdp.evaluate(COUNT_JS) or {}
                except RuntimeError:
                    time.sleep(2)
                    cdp = app_view()
                    continue
                if (after.get("songs") or 0) >= (base.get("songs") or 0) + 4 and after.get("queue") == 0:
                    break
                time.sleep(2)
            say(f"{name}: after the restart: {json.dumps(after)[:900]}")
            added = (after.get("songs") or 0) - (base.get("songs") or 0)
            check(f"{name}: all four tracks arrive after the restart", bool(first) and added >= 4,
                  f"first {bool(first)}; added {added}; queue left {after.get('queue')}")
            check(f"{name}: each once", added == 4, f"added {added}: {after.get('titles')}")
            fresh = [l for l in after.get("lists") or [] if l["title"] == ALBUM[2] and l["songs"] == 4]
            check(f"{name}: all four in one playlist", bool(fresh), str(after.get("lists")))
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
