#!/usr/bin/env python3
"""Capture the manual's screenshots by driving a headless Chromium over CDP.

**Why a driver rather than `chromium --screenshot`.** The app has no URL routing — which view is
open lives in `localStorage` — and `--screenshot` gives you one navigation with no way to set
anything first. `frame-ancestors 'none'` (correctly) rules out framing it, and `--virtual-time-budget`
does not survive a redirect. So the shots are taken the way a person takes them: open the page, set
the state, wait, click, capture.

Run it through `ci/shots.sh`, which builds the binary, serves a throwaway demo vault and cleans up.
"""
import base64, json, os, socket, struct, subprocess, sys, time, urllib.request

CHROME = os.environ.get("FM_SHOT_CHROME", "chromium")
PORT = int(os.environ.get("FM_SHOT_CDP_PORT", "9222"))
APP = os.environ.get("FM_SHOT_URL", "http://127.0.0.1:8791/")
OUT = os.environ.get("FM_SHOT_OUT", "docs/src/images")
# Snap-confined Chromium can only write inside $HOME, which is also why the profile lives here.
PROFILE = os.path.expanduser(os.environ.get("FM_SHOT_PROFILE", "~/snap/chromium/common/fm-shot"))


class WS:
    """The smallest WebSocket client that can speak CDP. Text frames only, no extensions."""

    def __init__(self, url):
        _, rest = url.split("://", 1)
        hostport, path = rest.split("/", 1)
        host, port = hostport.split(":")
        self.s = socket.create_connection((host, int(port)))
        key = base64.b64encode(os.urandom(16)).decode()
        self.s.sendall(
            (
                f"GET /{path} HTTP/1.1\r\nHost: {hostport}\r\nUpgrade: websocket\r\n"
                f"Connection: Upgrade\r\nSec-WebSocket-Key: {key}\r\n"
                "Sec-WebSocket-Version: 13\r\n\r\n"
            ).encode()
        )
        buf = b""
        while b"\r\n\r\n" not in buf:
            buf += self.s.recv(4096)
        self.buf = buf.split(b"\r\n\r\n", 1)[1]
        self.id = 0

    def _recv(self, n):
        while len(self.buf) < n:
            chunk = self.s.recv(65536)
            if not chunk:
                raise RuntimeError("websocket closed")
            self.buf += chunk
        out, self.buf = self.buf[:n], self.buf[n:]
        return out

    def send(self, method, **params):
        self.id += 1
        payload = json.dumps({"id": self.id, "method": method, "params": params}).encode()
        # Client frames must be masked; the mask is allowed to be zeros, which keeps this short.
        header = b"\x81"
        n = len(payload)
        if n < 126:
            header += struct.pack("!B", 0x80 | n)
        elif n < 65536:
            header += struct.pack("!BH", 0x80 | 126, n)
        else:
            header += struct.pack("!BQ", 0x80 | 127, n)
        self.s.sendall(header + b"\x00\x00\x00\x00" + payload)
        return self.id

    def recv(self):
        b1, b2 = struct.unpack("!BB", self._recv(2))
        n = b2 & 0x7F
        if n == 126:
            n = struct.unpack("!H", self._recv(2))[0]
        elif n == 127:
            n = struct.unpack("!Q", self._recv(8))[0]
        return json.loads(self._recv(n).decode())

    def call(self, method, **params):
        want = self.send(method, **params)
        while True:
            msg = self.recv()
            if msg.get("id") == want:
                if "error" in msg:
                    raise RuntimeError(f"{method}: {msg['error']}")
                return msg.get("result", {})


def launch(width, height):
    proc = subprocess.Popen(
        [CHROME, "--headless", "--disable-gpu", "--no-sandbox", "--hide-scrollbars",
         f"--remote-debugging-port={PORT}", f"--user-data-dir={PROFILE}",
         f"--window-size={width},{height}", "about:blank"],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )
    for _ in range(100):
        try:
            with urllib.request.urlopen(f"http://127.0.0.1:{PORT}/json/version", timeout=1):
                return proc
        except Exception:
            time.sleep(0.2)
    proc.kill()
    raise SystemExit("chromium did not open a debugging port")


def page(ws_url):
    ws = WS(ws_url)
    ws.call("Page.enable")
    ws.call("Runtime.enable")
    return ws


def target():
    with urllib.request.urlopen(f"http://127.0.0.1:{PORT}/json/list") as r:
        for t in json.load(r):
            if t["type"] == "page":
                return t["webSocketDebuggerUrl"]
    raise SystemExit("no page target")


def js(ws, expr):
    r = ws.call("Runtime.evaluate", expression=expr, awaitPromise=True, returnByValue=True)
    return r.get("result", {}).get("value")


def goto(ws, url, settle=2.0):
    ws.call("Page.navigate", url=url)
    time.sleep(settle)


def shot(ws, name, width, height):
    os.makedirs(OUT, exist_ok=True)
    r = ws.call("Page.captureScreenshot", format="png",
                clip={"x": 0, "y": 0, "width": width, "height": height, "scale": 1})
    path = os.path.join(OUT, f"{name}.png")
    with open(path, "wb") as f:
        f.write(base64.b64decode(r["data"]))
    print(f"  {path}  ({os.path.getsize(path) // 1024} KB)")


PANE = ('{"id":"p1","kind":"%s","groupBy":"status","agendaMode":"%s","timelineMode":"%s",'
        '"query":"","noteId":null,"viewName":null}')


def workspace(kind, agenda="month", timeline="feed"):
    return ('{"v":2,"cols":1,"layout":"single","active":0,"panes":[' +
            (PANE % (kind, agenda, timeline)) + "]}")


def show(ws, kind, agenda="month", timeline="feed", note=None, query=""):
    """Put one pane on screen and reload into it.

    Reload rather than click: the workspace is read once at mount, so setting it and reloading is
    both the shortest path and the one that cannot depend on a menu's markup staying put.
    """
    pane = json.loads(PANE % (kind, agenda, timeline))
    pane["noteId"] = note
    pane["query"] = query
    ws_json = json.dumps({"v": 2, "cols": 1, "layout": "single", "active": 0, "panes": [pane]})
    js(ws, "localStorage.setItem('fm-workspace', " + json.dumps(ws_json) + ")")
    ws.call("Page.reload")
    time.sleep(3.0)


def click(ws, selector, settle=1.5):
    """Click by selector through the page's own DOM.

    `Input.dispatchMouseEvent` would need coordinates, which means reading a bounding box and
    re-reading it whenever the layout moves. A `.click()` is what the element does anyway.
    """
    hit = js(ws, f"(() => {{ const e = document.querySelector({json.dumps(selector)});"
                 f" if (!e) return false; e.click(); return true; }})()")
    if not hit:
        raise SystemExit(f"no element matched {selector!r} — the markup moved, fix the selector")
    time.sleep(settle)


def click_text(ws, text, settle=1.5):
    """Click the first element whose visible text starts with `text`.

    For menu items, which carry no `aria-label` of their own — their accessible name *is* their
    text, so matching on it is matching on the same thing a screen reader would announce.
    """
    hit = js(ws, "(() => { const t = %s;"
                 " const e = Array.from(document.querySelectorAll('button,[role=menuitem],a'))"
                 "  .find(x => (x.textContent||'').trim().startsWith(t));"
                 " if (!e) return false; e.click(); return true; })()" % json.dumps(text))
    if not hit:
        raise SystemExit(f"no element whose text starts with {text!r}")
    time.sleep(settle)


def first_note_id(ws):
    """A note id from the running vault, so the read-view shot is of real content."""
    return js(ws, """(async () => {
      const r = await fetch('/api/recent', {method:'POST', headers:{'content-type':'application/json'}, body:'{}'});
      const rows = await r.json();
      const want = rows.find(n => (n.title || '').startsWith('Notation'));
      return (want || rows[0]).id;
    })()""")


def main():
    W, H = 1440, 900
    proc = launch(W, H)
    try:
        ws = page(target())
        # The viewport is set explicitly rather than left to `--window-size`: headless reports a
        # slightly smaller inner height, and the app is laid out in `dvh`, so without this every
        # shot has a strip of dead space under it.
        ws.call("Emulation.setDeviceMetricsOverride",
                width=W, height=H, deviceScaleFactor=1, mobile=False)
        # One navigation to establish the origin, then the state is set and the app reloaded —
        # which is exactly what a redirect could not do, because virtual time does not cross one.
        goto(ws, APP, 3.0)

        show(ws, "board");                     shot(ws, "board", W, H)
        show(ws, "agenda");                    shot(ws, "agenda", W, H)
        show(ws, "timeline");                  shot(ws, "timeline", W, H)
        show(ws, "timeline", timeline="compact"); shot(ws, "timeline-list", W, H)
        show(ws, "search", query="paper");     shot(ws, "search", W, H)

        note = first_note_id(ws)
        if note:
            show(ws, "note", note=note)
            shot(ws, "note", W, H)

        # The two panels a first-time reader is sent to, opened the way a person opens them.
        # Selected by `aria-label`, which is the app's own contract with assistive tech and is
        # therefore the attribute least likely to be restyled out from under this script.
        show(ws, "board")
        click(ws, 'button[aria-label="other backup options"]', 1.5)
        click_text(ws, "Backup options", 2.5)
        shot(ws, "backup", W, H)

        show(ws, "board")
        click(ws, 'button[aria-label="settings"]', 2.0)
        shot(ws, "settings", W, H)

        show(ws, "board")
        click(ws, 'button[aria-label="make something new"]', 1.5)
        shot(ws, "new", W, H)
    finally:
        proc.terminate()


if __name__ == "__main__":
    main()
