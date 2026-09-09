# Security

## Reporting a vulnerability

**Use GitHub's private vulnerability reporting**, on this repository:
**Security → Advisories → Report a vulnerability**
(<https://github.com/formicaria-org/formicaria/security/advisories/new>).

That channel is private between you and the maintainer, so a working exploit can be described in
full without publishing it first. **Please do not open a public issue for a vulnerability.**

This is a single-maintainer project, not a company with a rota. Expect an acknowledgement within a
week; if a week passes with nothing, assume the notification was missed rather than ignored, and
say so on the same advisory. There is no bug bounty.

When you report, the useful things are: what an attacker has to already have (a file in the vault?
a collaborator's push? a chosen filename?), what they get, and the smallest reproduction you can
manage. A version — Settings shows it, and it is the release tag — matters, because several of the
answers below have changed.

## What this software actually handles

Worth stating plainly, because it sets the stakes:

- **Your notes**, as Markdown files in a directory you chose, and the media beside them.
- **A git credential**, on any device with no `git` binary. Where git is installed its own
  credential helper holds the token and formicaria never sees it stored. Where it is not — the
  phone, and **a Windows machine with no git installed**, which is the ordinary case since the
  Windows build carries its own git — there is no helper, so the app writes the token itself,
  `0600`, beside the vault list. The test is the binary, not the platform
  (`fm_app::secrets::should_store`).
- **A restic password** (`RESTIC_PASSWORD`, or the stored one), which is the key to your backups.
  It is written `0600` beside the vault list and **never into it**, and every API that reports on
  backups returns it only as a boolean — never by value.
- **A local model**, if you enable the study assistant, plus whatever it fetched.

## The threat model, stated honestly

**The server binds `127.0.0.1` and has no authentication.** Anything that can make an HTTP request
from your machine can drive it, and that includes a page in your browser making a cross-origin
request to `127.0.0.1:8765`. This is a single-user local application; it is not designed to be
exposed to a network, and putting it behind a reverse proxy does not make it multi-user.

**A vault is an audience.** Access is determined by *which git repository a note is in*, never by a
field inside the file — a value in a file can be mistyped, and git history is permanent. If two
people should not read the same note, the note belongs in a different repository. There is no
per-note permission and there will not be one.

**A shared vault means running a collaborator's content.** Note bodies from a shared vault are
rendered on your machine. They are treated as untrusted: the HTML sink is audited by a CI grep,
Mermaid never runs with `securityLevel: 'loose'`, and the read view's content security policy is
narrow. A hole in any of that is squarely in scope.

**The study assistant is opt-in and local.** It runs on your device, and it can only reply, or
propose a change on a review branch — it can never write to `main`. A build without it contains
none of it.

## In scope

Anything that lets one of the following happen:

- reading or writing a vault the caller should not have — across vaults, or from a web page
- a note's *content* causing code to run, or reaching something outside its own rendering
- a credential or the restic password leaking: into a note, into git, into a log, into an API
  response, or into a file that is not `0600`
- a path in a note or an asset reference escaping the vault directory
- a merge or a pull destroying text that was not in conflict — this project's stance is that
  **both versions go where a human can see them, never a silent choice**
- the release archive or an update path carrying something it should not, or being trivially
  substitutable

## Out of scope

- **The lack of authentication on `127.0.0.1`.** Documented above and by design.
- **Anything requiring an attacker who already has your user account.** They have your notes
  directly; the app is not the boundary.
- Denial of service against your own local server.
- Vulnerabilities in `git`, `restic`, `pdftotext` or `vipsthumbnail` themselves. They are shelled
  out to deliberately, and they are yours to keep updated — though *how* we invoke them is in
  scope, and a shell-injection through a filename very much is. **libgit2 is the exception**: on
  Windows, Android and iOS it is compiled into the binary rather than shelled out to, so a libgit2
  advisory is ours to ship a fix for and is in scope.
- Reports from an automated scanner with no demonstrated impact on this application.

## What ships in the binary

[`THIRD-PARTY.md`](THIRD-PARTY.md) lists every crate, npm package and font bundled into a release,
regenerated from the real dependency tree and checked for staleness by the build gate. If an
advisory lands on something in there, that file is where to look first. `cargo deny` gates the Rust
half on every gate run, for advisories as well as licences.
