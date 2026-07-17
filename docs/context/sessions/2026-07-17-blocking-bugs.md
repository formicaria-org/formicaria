# 2026-07-17 — Three blocking bugs from the audit

**Outcome:** fixed the three high-severity bugs a verified audit turned up hours
after the two-tier backup shipped — two of them in that very feature, and one a
pre-existing hole that quietly voided its central guarantee. `pixi run ci` green;
`tests/git.rs` 7 → 9; the `Origin` fix proven against a live server.

## What was wrong

**1. `ensure_repo` skipped the ignore rules on a repo it did not create.**
It returned `Ok(false)` the moment `vault/.git` existed — *before* calling
`write_gitignore`. So a vault someone `git init`ed by hand never got ignore rules,
and the next `commit_all` (`git add -A`) would sweep `blobs/` and `index.sqlite`
into history: a push would then ship every PDF and video to the remote, voiding
the "notes tier is light" promise entirely. Pre-existing, but it aimed straight at
the feature shipped the same day. **The owner's own vault was never affected** — it
has the `.gitignore`, 0 blobs tracked, because formicarium created it. The bug
bites the *next machine you clone to*, which is the restore path.
Fix: call `write_gitignore` before the early return; it is already idempotent, so
a cloned vault's tracked `.gitignore` is untouched.

**2. `push_squashed` destroyed history before a push designed to fail.**
`reset --soft` + commit ran *before* `git push`. A rejected push — the diverged
remote this function exists to make safe — left the granular history already
collapsed and nothing pushed: the user paid the cost and got no backup. Mine, from
the same morning. The original tests missed it because that scenario had a single
unpushed commit, so no squash occurred.
Fix: capture `rev-parse HEAD` before squashing; restore with `reset --soft` if the
push fails. `--soft` is exact here — the index already holds that tree.

**3. No `Origin` check: any website could drive the vault.**
`POST /api/*` had no origin validation, so any page open in the browser could call
`/api/delete` — and a `text/plain` body skips the CORS preflight, so the browser
hides the *response* while the side effect lands. The most serious finding of the
audit, and no document recorded it.
Fix: allowed origins built from `FM_ADDR` on `AppState`; a **present and
mismatched** `Origin` is 403. **Absent `Origin` passes** — that is a non-browser
client (curl, scripts, tests), not a CSRF vector, since there are no ambient
credentials to abuse; browsers always send `Origin` on POST, same-origin included.

**4. A doc overclaim, mine.** `backup.md` said a git-only restore recovers
`manifest.json` so `fm verify` names your missing media. Only `fm manifest` ever
writes that file — the app never does. The conclusion survives (`verify.rs` finds
missing blobs from the notes' own `asset:` references) but the mechanism was wrong.
Corrected.

## Verified

`tests/git.rs` gained two regression tests: a hand-`init`ed repo still gets
`blobs/` ignored; and a diverged remote rejects the push **with the history
restored** (two clones of a local bare repo force the real rejection — no network,
no credentials). The second is a true regression test: without the restore the
commit count drops by one and the assertion fires.

The `Origin` fix was proven against a live `fm-serve`: `Origin: https://evil.com`
POSTing `delete` → **403, note survives**; both `127.0.0.1` and `localhost`
spellings of our own origin → 200; bare `curl` with no `Origin` → 200.

## How these were found

A verified audit (146 agents; every claimed issue re-checked against the code
rather than trusted from the docs). 136 of 138 documented issues were still open —
the docs are honest — but the value was the 8 items **no document recorded**,
including all three above. The cost was disproportionate (~4.3M tokens for what
was largely a three-document read), and the owner ruled afterwards that the
minimal-token rule outranks any effort setting. The finding stands; the method
does not.

## Left not-working

The audit's other new findings are now in `known-issues.md`, unfixed: one malformed
`.md` bricks startup; `fm-serve` has no tests at all; a backgrounded tab can shut
the app down (3s heartbeat vs a 10s watchdog, throttled to ~1/min); SQLite has no
`busy_timeout` so the CLI races the server. Phase 1 of the collaboration plan (the
lost-update guard — `MASTERPLAN.md:319` claims an mtime check `update_body` does not
do) is **not started**; it is the next thing and it blocks any shared-vault work.
