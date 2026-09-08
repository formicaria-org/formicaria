"""The app speaks the user's words, not git's — enforced.

**The rule** (`decisions.md`, 2026-09-08, `#ui`): text a person reads never uses git's vocabulary.
No *push*, *pull*, *commit*, *remote*, *branch*, *HEAD*, *origin*, *fetch*, *rebase*, *clone*,
*upstream*. Say what the thing means to someone who keeps notes: *sent* / *not sent yet* / *saved
here* / *their changes* / *where your notes are copied to*.

**Why it needs a check at all.** The rule is about wording, and wording is exactly the kind of thing
that is obeyed on the day it is written and forgotten by the third new string. When this was first
applied, 28 strings across five components were in git-speak — including the one that mattered most,
*"125 commits not pushed"*, which is the app's only report of work that has not left the device.
Nothing had noticed, because nothing was looking.

**Why the scan is this fussy.** `git` words are everywhere in this codebase legitimately: variable
names (`remoteDrafts`, `v.remote`), API calls (`fetch`, `.push(`), mode values (`'clone'`), CSS
(`pushed to the right`), and — most of all — maintainer comments, which *should* say `commit`
because they are for the person reading the code. Matching those would make the guard noise, and a
noisy guard gets disabled. So this looks **only at what is rendered**:

  - text nodes in markup (`>…<`), outside `<script>` and `<style>`,
  - the three attributes a user actually reads: `title`, `aria-label`, `placeholder`,
  - `text:` strings in script, which is how `BackupPanel` builds its step-by-step report.

and it strips comments and `${…}` interpolations first, so `${shortDest(v.remote)}` — an expression,
not prose — does not trip it.

**Verified to fire**, not merely to pass: injecting a violation of each of the four shapes above was
caught 4/4, with 0 false positives across every `.svelte` file in the app. A guard that has never
been seen to fail is not known to be a guard.

**The deliberate exception is diagnosis.** An error detail, a log line or a debug pane may name the
git thing, because there the literal word is the useful one. Those reach the user through
interpolated error values (`${msg(e)}`, `${syncFor(v).error}`), which this scanner strips along with
every other expression — so the exception is structural, and needs no allowlist to maintain.
"""

import pathlib
import re
import sys

# Pure git vocabulary. Deliberately excludes "merge" and "conflict": those are ordinary English for
# what actually happens to a note, the app has always used them in user text, and the owner's own
# reports use them back. The list is what a person would have to learn *git* to understand.
WORDS = re.compile(
    r"\b(push|pushed|pushes|pushing|pull|pulled|pulling|commit|commits|committed|committing"
    r"|remote|remotes|branch|branches|HEAD|origin|fetch|fetched|rebase|rebased|upstream"
    r"|unpushed|clone|cloned|cloning|checkout|stash)\b"
)

COMMENTS = (
    re.compile(r"<!--.*?-->", re.S),
    re.compile(r"/\*.*?\*/", re.S),
    re.compile(r"^[ \t]*//.*$", re.M),
)
INTERPOLATION = re.compile(r"\$\{[^{}]*\}")
SCRIPT = re.compile(r"<script[^>]*>(.*?)</script>", re.S)
STYLE = re.compile(r"<style[^>]*>.*?</style>", re.S)
ATTRS = re.compile(r"""(?:title|aria-label|placeholder)=(?:"([^"]*)"|\{`([^`]*)`\})""")
TEXT_NODE = re.compile(r">([^<>{}]{4,})<")
STEP_TEXT = re.compile(r"\btext:\s*(`(?:[^`\\]|\\.)*`|'[^']*')", re.S)


def strip(src: str) -> str:
    for pattern in COMMENTS:
        src = pattern.sub("", src)
    return INTERPOLATION.sub("", src)


def rendered(src: str):
    """Every fragment of this component that a person actually reads."""
    src = STYLE.sub("", src)
    script = "\n".join(SCRIPT.findall(src))
    markup = strip(SCRIPT.sub("", src))
    for a, b in ATTRS.findall(markup):
        yield a or b
    yield from TEXT_NODE.findall(markup)
    yield from STEP_TEXT.findall(strip(script))


def main() -> int:
    root = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "ui/src")
    hits = []
    for path in sorted(root.rglob("*.svelte")):
        for fragment in rendered(path.read_text(encoding="utf-8")):
            word = WORDS.search(fragment)
            if word:
                hits.append((path, word.group(0), " ".join(fragment.split())[:100]))
    for path, word, fragment in hits:
        print(f"  {path}: “{word}” in: {fragment}")
    return 1 if hits else 0


if __name__ == "__main__":
    sys.exit(main())
