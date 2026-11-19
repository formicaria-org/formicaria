#!/usr/bin/env python3
"""**A privacy test site** (`docs/context/privacy-plan-2026-11-19.md` 1.1), served by the checks on this
machine: a page that loads real trackers and an ad, a link carrying tracking parts and a tracker's
redirect, `/echo` answering the request's own headers (Global Privacy Control), and a service worker
that asks a tracker too. What each protection did is read back by the check that opened it.

`python3 ci/privacy_site.py <port>` serves until stopped. The page's own scripts set `window.fm`:
`{app: 1}` always, `ad`, `ga`, `fb`, `sw` when those loaded. The trackers are real (Google Analytics,
Facebook's pixel): a blocker that lets them through lets the page record it, nothing else is sent.
"""
import json
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

PAGE = """<!doctype html><meta charset="utf-8"><title>privacy test</title>
<script>window.fm = {app: 0}; window.fmMark = (k) => { window.fm[k] = 1; };</script>
<script src="/app.js"></script>
<script src="/ads/custom_ads.js"></script>
<script src="https://www.google-analytics.com/analytics.js" onload="fmMark('ga')"></script>
<script src="https://connect.facebook.net/en_US/fbevents.js" onload="fmMark('fb')"></script>
<div class="ad-banner" id="ad_banner">an ad box</div>
<p id="story">The story.</p>
<a id="tracked" href="{other}/landing.html?id=7&utm_source=mail&fbclid=abc123&gclid=x">a tracked link</a>
<a id="wrapped" href="https://www.google.com/url?q={other_enc}%2Flanding.html%3Fid%3D8&sa=t">a wrapped link</a>
<script>
if (navigator.serviceWorker) navigator.serviceWorker.register('/sw.js').then(() => 0, () => 0);
fetch('/echo').then(r => r.json()).then(h => { window.fm.headers = h; }, () => 0);
</script>
"""

SW = """self.addEventListener('install', () => self.skipWaiting());
self.addEventListener('activate', (e) => e.waitUntil(clients.claim()));
fetch('https://www.google-analytics.com/collect?v=1&t=sw').then(() => fetch('/sw-reached'), () => 0);
"""


class Handler(BaseHTTPRequestHandler):
    sw_reached = 0

    def log_message(self, *a):  # quiet
        pass

    def send(self, body: str, kind: str = "text/html; charset=utf-8"):
        b = body.encode()
        self.send_response(200)
        self.send_header("Content-Type", kind)
        self.send_header("Content-Length", str(len(b)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(b)

    def do_GET(self):
        path = self.path.split("?")[0]
        if path in ("/", "/page.html"):
            q = dict(p.split("=", 1) for p in self.path.split("?", 1)[1].split("&") if "=" in p) if "?" in self.path else {}
            other = q.get("other", f"http://localhost:{self.server.server_port}")
            other = other.replace("%3A", ":").replace("%2F", "/")
            enc = other.replace(":", "%3A").replace("/", "%2F")
            self.send(PAGE.replace("{other_enc}", enc).replace("{other}", other))
        elif path == "/app.js":
            self.send("fmMark('app');", "text/javascript")
        elif path == "/ads/custom_ads.js":
            self.send("fmMark('ad');", "text/javascript")
        elif path == "/sw.js":
            self.send(SW, "text/javascript")
        elif path == "/sw-reached":
            Handler.sw_reached += 1
            self.send("ok", "text/plain")
        elif path == "/sw-count":
            self.send(json.dumps({"reached": Handler.sw_reached}), "application/json")
        elif path == "/echo":
            self.send(json.dumps({k.lower(): v for k, v in self.headers.items()}), "application/json")
        elif path == "/landing.html":
            self.send("<!doctype html><title>landing</title><p id=where></p>"
                      "<script>document.getElementById('where').textContent=location.href</script>")
        else:
            self.send_response(404)
            self.end_headers()


if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8794
    ThreadingHTTPServer(("0.0.0.0" if "--all" in sys.argv else "127.0.0.1", port), Handler).serve_forever()
