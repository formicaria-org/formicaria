#!/usr/bin/env python3
"""Open imported notes in a real browser and check that what they link to is *visible*.

Layer 3 of `docs/context/logseq-import-plan-2026-10-14.md`. The Rust conformance test measures what
the importer wrote; this measures what a reader sees: every image loaded, every linked file
answering, no Logseq syntax left on screen. It also writes a contact sheet of screenshots, so the
owner can look rather than trust a number.

Run it through `ci/import-render.sh`, which serves the imported vault on a spare port. Reuses the
CDP driver in `ci/shots.py` — one WebSocket client in the repo is enough.
"""
import html, json, os, sys, time

OUT = os.environ.setdefault("FM_SHOT_OUT", "target/import-corpus/shots")
import shots  # noqa: E402  (reads FM_SHOT_* at import)

W, H = 1280, 900
LIMIT = int(os.environ.get("FM_RENDER_SAMPLE", "60"))
# What Logseq syntax looks like when it was not converted. Checked against the *rendered* text.
DEBRIS = ["[[", "{{", "](", "{:height", "../assets/", "%3A"]


def api(ws, cmd, args):
    return shots.js(ws, "(async () => { const r = await fetch('/api/%s', {method:'POST',"
                        " headers:{'content-type':'application/json'}, body:%s});"
                        " return r.ok ? await r.json() : null; })()" % (cmd, json.dumps(json.dumps(args))))


def open_note(ws, note_id):
    pane = json.loads(shots.PANE % ("note", "month", "feed"))
    pane["noteId"] = note_id
    ws_json = json.dumps({"v": 2, "cols": 1, "layout": "single", "active": 0, "panes": [pane]})
    shots.js(ws, "localStorage.setItem('fm-workspace', " + json.dumps(ws_json) + ")")
    ws.call("Page.reload")
    # Ready when the read view exists and its content has stopped changing for a second — the
    # asset, note and embed passes are asynchronous, so "exists" alone measures a half-rendered
    # note (seen: the image count moved between two runs over the same vault).
    last, stable = None, 0
    for _ in range(60):
        time.sleep(0.25)
        now = shots.js(ws, "(() => { const r = document.querySelector('.read');"
                           " return r ? r.innerHTML.length + ':' + r.querySelectorAll('img').length : null; })()")
        if now is not None and now == last:
            stable += 1
            if stable >= 4:
                return True
        else:
            stable = 0
        last = now
    return last is not None


INSPECT = """(async () => {
  const r = document.querySelector('.read');
  if (!r) return null;
  const imgs = [...r.querySelectorAll('img')];
  // The read view lazy-loads images, so one below the fold has not loaded *yet* — which is not
  // broken. Load each one now and wait for it (or 8 s), then judge.
  await Promise.all(imgs.map(i => {
    i.loading = 'eager';
    return i.complete && i.naturalWidth > 0 ? null : Promise.race([
      new Promise(res => { i.addEventListener('load', res, {once: true});
                           i.addEventListener('error', res, {once: true}); }),
      new Promise(res => setTimeout(res, 8000)),
    ]);
  }));
  const links = [...r.querySelectorAll('a.asset-link')];
  const dead = [];
  for (const a of links) {
    try {
      const res = await fetch(a.href, {headers: {Range: 'bytes=0-0'}});
      if (!res.ok) dead.push(a.textContent + ' → ' + res.status);
    } catch (e) { dead.push(a.textContent + ' → ' + e); }
  }
  // Stray syntax is judged on prose only: code shows what it says (`[this](){…}` is C++), and a
  // link's text may be a URL or a page name that really contains `%%3A` or `[[`.
  const prose = r.cloneNode(true);
  prose.querySelectorAll('pre, code, a, .katex').forEach(e => e.remove());
  document.body.appendChild(prose);
  const text = prose.innerText;
  prose.remove();
  // An image from somebody else's website was never part of the graph; it is counted, not judged.
  const local = i => !/^https?:/i.test(i.getAttribute('src') || '') || i.src.startsWith(location.origin);
  return {
    images: imgs.length,
    broken: imgs.filter(i => local(i) && i.naturalWidth === 0).map(i => i.alt || i.src),
    remoteBroken: imgs.filter(i => !local(i) && i.naturalWidth === 0).map(i => i.src),
    files: links.length,
    dead,
    missing: [...r.querySelectorAll('.asset-missing-inline')].map(e => e.textContent),
    katexErrors: r.querySelectorAll('.katex-error').length,
    debris: %s.filter(p => text.includes(p)),
  };
})()""" % json.dumps(DEBRIS)


def main():
    proc = shots.launch(W, H)
    results = []
    try:
        ws = shots.page(shots.target())
        shots.goto(ws, shots.APP, settle=3.0)
        notes = api(ws, "recent", {}) or []
        bodies = {}
        for n in notes:
            got = api(ws, "get", {"id": n["id"]})
            bodies[n["id"]] = (got or {}).get("body", "")
        # Every note that references an attachment, plus an even sample of the rest.
        with_assets = [n for n in notes if "asset:" in bodies[n["id"]]]
        rest = [n for n in notes if "asset:" not in bodies[n["id"]]]
        step = max(1, len(rest) // max(1, LIMIT))
        chosen = with_assets + rest[::step][:LIMIT]
        print(f"import-render: {len(notes)} notes, {len(with_assets)} with attachments,"
              f" checking {len(chosen)}")
        os.makedirs(OUT, exist_ok=True)
        for i, n in enumerate(chosen):
            ok = open_note(ws, n["id"])
            r = shots.js(ws, INSPECT) if ok else None
            r = r or {"error": "the note did not open"}
            r["id"], r["title"] = n["id"], n.get("title") or ""
            r["shot"] = f"{i:04d}.png"
            shots.shot(ws, f"{i:04d}", W, H)
            results.append(r)
    finally:
        proc.kill()

    bad = [r for r in results
           if r.get("error") or r.get("broken") or r.get("dead") or r.get("missing")
           or r.get("katexErrors") or r.get("debris")]
    summary = {
        "checked": len(results),
        "images": sum(r.get("images", 0) for r in results),
        "imagesBroken": sum(len(r.get("broken", [])) for r in results),
        "remoteImagesNotLoading": sum(len(r.get("remoteBroken", [])) for r in results),
        "files": sum(r.get("files", 0) for r in results),
        "filesDead": sum(len(r.get("dead", [])) for r in results),
        "missing": sum(len(r.get("missing", [])) for r in results),
        "katexErrors": sum(r.get("katexErrors", 0) for r in results),
        "notesWithDebris": sum(1 for r in results if r.get("debris")),
        "notesFailing": len(bad),
    }
    with open(os.path.join(OUT, "render-report.json"), "w") as f:
        json.dump({"summary": summary, "failing": bad, "all": results}, f, indent=2)

    # The contact sheet: failures first, each captioned with what was wrong.
    cards = []
    for r in sorted(results, key=lambda r: r not in bad):
        why = "; ".join(f"{k}: {r[k]}" for k in ("error", "broken", "dead", "missing", "debris")
                        if r.get(k)) or "ok"
        cards.append(f'<figure class="{"bad" if r in bad else "ok"}"><a href="{r["shot"]}">'
                     f'<img loading="lazy" src="{r["shot"]}"></a><figcaption><b>'
                     f'{html.escape(r["title"])}</b><br>{html.escape(why)}</figcaption></figure>')
    with open(os.path.join(OUT, "index.html"), "w") as f:
        f.write("<!doctype html><meta charset=utf-8><title>Import render check</title><style>"
                "body{font:14px system-ui;margin:16px;background:#fafafa}"
                "main{display:grid;grid-template-columns:repeat(auto-fill,minmax(320px,1fr));gap:12px}"
                "figure{margin:0;background:#fff;border:2px solid #ccc;border-radius:6px;padding:6px}"
                "figure.bad{border-color:#c33}img{width:100%}figcaption{overflow-wrap:anywhere}"
                f"</style><h1>Import render check</h1><pre>{html.escape(json.dumps(summary, indent=2))}"
                f"</pre><main>{''.join(cards)}</main>")

    print(json.dumps(summary, indent=2))
    print(f"import-render: contact sheet at {OUT}/index.html")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
