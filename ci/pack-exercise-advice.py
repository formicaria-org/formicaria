#!/usr/bin/env python3
"""Make the exercise cards' safety tips and further links, and add the new films, from the 2026-10-18 research.

    python3 ci/pack-exercise-advice.py docs/context/exercise-tips-draft-2026-10-18.json \\
        crates/fm-app/src/shipped/exercise-advice.json crates/fm-app/src/shipped/exercise-videos.json

**What it is.** A maintainer runs this by hand, like `pack-exercises.py`. The app never runs it;
the build reads only the JSON it leaves behind, which is committed. The research behind it is
`docs/context/exercise-tips-research-2026-10-18.md`; the draft it reads is that run's output.

**What it writes.**

1. `exercise-advice.json`, two things:
   - `safely` — the **Safety Tips** box of the National Institute on Aging's booklet *Workout to
     Go: A Sample Exercise Routine from the National Institute on Aging at NIH* (2015, on
     govinfo), **whole and word for word**: each tip's bold line (`head`) and the sentence(s)
     under it (`text`). Shown from every card that needs a bar, free weights or a machine.
     Quoting the box whole, and not single lines of it on single cards, is the research's own
     recommendation: the same four lines would otherwise repeat on 46 cards.
   - `links` — per card, pages elsewhere on how the exercise is done and what to avoid
     (the American Council on Exercise's library, the NHS, the NIA). **Links only**: ACE's terms
     forbid copying, and nothing of any page is taken but its title.
2. New entries appended to `exercise-videos.json`, then the whole list put in the owner's order
   (a known body's that is not the Army's, a trainer whose qualification was confirmed, a trainer
   with nothing confirmed, the Army's last — `decisions.md#data`, 2026-10-12). The list was already
   in that order, so only the new entries move.

**What is checked, here and not on trust.** The booklet is downloaded and its box parsed out of
`pdftotext -layout`; changed are only whitespace and the PDF's ligature glyphs (`ﬁ` → `fi`), which
are a font's, not the NIA's spelling. Every link is requested and must answer 200; an HTML page's
own `<title>` must share a word with the title shown. Every YouTube film is asked of YouTube's
oEmbed, and its title and channel are taken from the answer, not from the draft.

**What is left out** (the research's checker, and this script's own reading): `EXCLUDE` below.

**Licence.** The NIA, in its words (https://www.nia.nih.gov/about/policies, read 2026-10-18; that
site now answers scripts with a "Human Verification" page, so this script cannot re-read it):
"Unless otherwise indicated, text within materials on the NIA website is in the public domain and
may be reused without our permission. Credit the “National Institute on Aging, National Institutes
of Health” as the source." The booklet carries no other notice. Its photographs are not taken.

Standard library only, plus `pdftotext` (Poppler) for the booklet.
"""
import html
import json
import re
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request

USER_AGENT = "formicaria-exercise-pack (https://github.com/formicaria-org/formicaria)"
PAUSE_S = 1.0
RETRY_PAUSE_S = 5.0
BOOKLET_URL = ("https://www.govinfo.gov/content/pkg/GOVPUB-HE20-PURL-gpo62941/pdf/"
               "GOVPUB-HE20-PURL-gpo62941.pdf")
BOOKLET = ("Workout to Go: A Sample Exercise Routine from the National Institute on Aging at NIH "
           "(2015)")
NIA_POLICY = "https://www.nia.nih.gov/about/policies"
NIA_LICENCE = ("Unless otherwise indicated, text within materials on the NIA website is in the "
               "public domain and may be reused without our permission.")
SAFE_FOR = ["bar", "free weights", "machine"]
MIN_TIPS = 7  # the box has seven; fewer means the booklet changed under the script

# (card id, url) pairs left out, and why.
EXCLUDE = {
    ("army-side-bridge", "https://www.acefitness.org/resources/everyone/exercise-library/303/side-plank/"):
        "a side plank on a BOSU dome, not on the floor (checker: wrong exercise)",
    ("army-single-leg-push-up", "https://www.youtube.com/watch?v=aVW1vXEvPeY"):
        "a lifestyle brand's channel, no qualification checked (checker: not reputable)",
    ("nhs-simple-grapevine", "https://www.youtube.com/watch?v=VqPn7Gfc4MY"):
        "presenter's qualification never checked (checker: not reputable)",
    ("nhs-sideways-walking", "https://www.youtube.com/watch?v=jbCscAwT8Ts"):
        "the DPT credential seen only in search snippets (checker: not reputable)",
    ("nhs-upper-body-twist", "https://www.youtube.com/watch?v=FpIfMyfpCk0"):
        "described only as 'a back stretch'; its match to the twist unconfirmed (checker)",
}
# A link to a page about exercise in general, not this one, is left out wherever it is.
GENERAL_PAGES = {
    "https://www.nia.nih.gov/health/exercise-and-physical-activity/three-types-exercise-can-improve-your-health-and-physical",
    "https://www.nia.nih.gov/health/exercise-and-physical-activity/four-types-exercise-can-improve-your-health-and-physical",
}
BY = [  # a link's body, by its address
    ("acefitness.org", "American Council on Exercise"),
    ("nhs.uk", "NHS website"),
    ("nia.nih.gov", "National Institute on Aging"),
    ("govinfo.gov", "National Institute on Aging"),
    ("cdc.gov/physicalactivity/downloads/growing_stronger", "Tufts University and the CDC"),
]
# A title as the draft gave it, less the site's own furniture.
TITLE_FURNITURE = re.compile(
    r"\s*(?:[|—–-]\s*(?:Exercise Library\s*\|\s*)?ACE(?: Exercise Library)?|\(ACE\)|"
    r"\|\s*Exercise Library\s*\|\s*ACE|Exercise Guide)\s*$")
TITLE_SECTION = re.compile(r"^[A-Z][A-Za-z &]+ Exercises \|\s*")

_last = [0.0]


def fetch(url, binary=False):
    """(status, body): one request at a time, a pause before each, one retry on a dropped line."""
    for attempt in (1, 2):
        wait = PAUSE_S - (time.monotonic() - _last[0])
        if wait > 0:
            time.sleep(wait)
        request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
        try:
            with urllib.request.urlopen(request, timeout=120) as response:
                data = response.read()
                return response.status, (data if binary else data.decode("utf-8", "replace"))
        except urllib.error.HTTPError as error:
            return error.code, b"" if binary else ""
        except (urllib.error.URLError, OSError) as error:
            if attempt == 2:
                return 0, f"{error}"
            time.sleep(RETRY_PAUSE_S)
        finally:
            _last[0] = time.monotonic()


def tidy(text):
    return re.sub(r"\s+", " ", text.replace("ﬁ", "fi").replace("ﬂ", "fl")).strip()


COMMON = {"how", "the", "and", "for", "with", "ace", "exercise", "exercises", "library", "guide"}


def words(text):
    return set(re.findall(r"[a-z]{3,}", text.lower())) - COMMON


def safety_tips(pdf):
    """The Safety Tips box: a `■■` line opens a tip, its next line is the bold head, the rest
    is the text, and the box ends at WARM UP."""
    text = subprocess.run(["pdftotext", "-layout", pdf, "-"], capture_output=True, text=True,
                          check=True).stdout
    start = text.index("Safety Tips")
    end = text.index("WARM UP", start)
    tips = []
    for block in text[start:end].split("■■")[1:]:
        lines = [line.strip() for line in block.strip().splitlines() if line.strip()]
        tips.append({"head": tidy(lines[0]), "text": tidy(" ".join(lines[1:]))})
    return tips


def body_of(url):
    return next((by for key, by in BY if key in url), None)


def page_title(own):
    """A page's own `<title>`, less the site's furniture: "Arm Exercises | Chest Press" and
    "Deadlift | Exercise Library | ACE" are "Chest Press" and "Deadlift"; "… - NHS" loses " - NHS"."""
    own = re.sub(r"\s+-\s+NHS\s*$", "", tidy(own))
    parts = [p.strip() for p in own.split("|")]
    kept = [p for p in parts if p and not re.fullmatch(r"ACE|Exercise Library", p)]
    named = [p for p in kept if not re.fullmatch(r".+ Exercises", p)]  # a section of the site
    return " | ".join(named or kept) or own


def clean_title(title):
    title = TITLE_SECTION.sub("", TITLE_FURNITURE.sub("", title.strip()))
    return re.sub(r"\s*[|—–]\s*$", "", title).strip()


def links(cards, left_out):
    out = {}
    for card in cards:
        kept = []
        for link in card["links"]:
            url = link["url"]
            if (card["id"], url) in EXCLUDE:
                left_out.append(f"{card['id']}: {EXCLUDE[(card['id'], url)]}")
                continue
            if url in GENERAL_PAGES:
                left_out.append(f"{card['id']}: {url} is about exercise in general")
                continue
            by = body_of(url)
            if not by:
                left_out.append(f"{card['id']}: {url} is from no body this script knows")
                continue
            status, page = fetch(url, binary=url.endswith(".pdf"))
            if status != 200:
                left_out.append(f"{card['id']}: {url} answered {status}")
                continue
            title = clean_title(link["title"])
            if not url.endswith(".pdf"):
                found = re.search(r"<title[^>]*>(.*?)</title>", page, re.S)
                own = page_title(html.unescape(found.group(1))) if found else ""
                if not words(own) & words(title):
                    left_out.append(f"{card['id']}: {url}'s own title {own!r} is not {title!r}")
                    continue
                title = own  # the page's words, not the draft's
            kept.append({"title": title, "url": url, "by": by})
        if kept:
            out[card["id"]] = kept
    return out


def rank(video):
    if "army" in video["channel"].lower():
        return 3
    return {"body": 0, "expert": 1, "trainer": 2}[video["tier"]]


def films(cards, names, videos, left_out, seen):
    known = {v["url"] for v in videos}
    credentials = {v["channel"]: (v.get("credential"), v.get("credential_url"))
                   for v in videos if v.get("credential")}
    confirmed = {v["channel"]: v["confirmed"] for v in videos if v.get("confirmed")}
    added = 0
    for card in cards:
        for film in card["films"]:
            url = film["url"]
            if (card["id"], url) in EXCLUDE:
                left_out.append(f"{card['id']}: {EXCLUDE[(card['id'], url)]}")
                continue
            if url in known:
                continue
            entry = {"exercise": names[card["id"]], "url": url}
            status, answer = fetch("https://www.youtube.com/oembed?format=json&url="
                                   + urllib.parse.quote(url, safe=""))
            if status != 200:
                left_out.append(f"{card['id']}: {url} — YouTube answered {status}")
                continue
            meta = json.loads(answer)
            entry["title"], entry["channel"] = meta["title"], meta["author_name"]
            entry["tier"] = film["tier"]
            if film["tier"] == "expert":
                if entry["channel"] not in credentials:
                    left_out.append(f"{card['id']}: {url} — {entry['channel']} has no "
                                    "qualification read before")
                    continue
                entry["credential"], entry["credential_url"] = credentials[entry["channel"]]
            if film["tier"] == "body":
                # Confirmed only where the channel was confirmed from the body's website before.
                entry["confirmed"] = entry["channel"] in confirmed
            views = re.sub(r"\D", "", str(film.get("views") or ""))
            entry["views"] = int(views) if views else None
            entry["seconds"] = None
            entry["note"] = f"Found by the research of {seen}; seen to exist that day."
            entry["cards"] = [card["id"]]
            videos.append(entry)
            known.add(url)
            added += 1
    videos.sort(key=rank)  # stable: the list was in this order, so only the new entries move
    return added


def main(argv):
    if len(argv) != 4:
        print("usage: pack-exercise-advice.py <draft.json> <exercise-advice.json> "
              "<exercise-videos.json>", file=sys.stderr)
        return 2
    draft_path, advice_path, videos_path = argv[1:]
    if not shutil.which("pdftotext"):
        print("pdftotext (Poppler) is needed for the NIA booklet", file=sys.stderr)
        return 2
    with open(draft_path, encoding="utf-8") as f:
        cards = json.load(f)["cards"]
    exercises_path = videos_path.replace("exercise-videos.json", "exercises.json")
    with open(exercises_path, encoding="utf-8") as f:
        names = {e["id"]: e["name"] for e in json.load(f)["exercises"]}
    unknown = [c["id"] for c in cards if c["id"] not in names]
    if unknown:
        print(f"the draft names cards the app does not carry: {unknown}", file=sys.stderr)
        return 1

    status, pdf = fetch(BOOKLET_URL, binary=True)
    if status != 200:
        print(f"the booklet answered {status}", file=sys.stderr)
        return 1
    with tempfile.NamedTemporaryFile(suffix=".pdf") as handle:
        handle.write(pdf)
        handle.flush()
        tips = safety_tips(handle.name)
    if len(tips) < MIN_TIPS or not all(t["head"] and t["text"] for t in tips):
        print(f"{len(tips)} safety tips read from the booklet — it changed; nothing written",
              file=sys.stderr)
        return 1

    seen = time.strftime("%Y-%m-%d")
    left_out = []
    # A film on a body's own page and not on YouTube is a page to read: it goes with the links.
    for card in cards:
        card["links"] += [f for f in card["films"] if "youtube.com" not in f["url"]]
        card["films"] = [f for f in card["films"] if "youtube.com" in f["url"]]
    advice = {
        "seen": seen,
        "safely": {
            "title": "Safety Tips",
            "by": "National Institute on Aging",
            "credit": "Text from the National Institute on Aging, National Institutes of Health",
            "work": BOOKLET,
            "url": BOOKLET_URL,
            "licence": NIA_LICENCE,
            "licence_url": NIA_POLICY,
            "for": SAFE_FOR,
            "tips": tips,
        },
        "links": links(cards, left_out),
    }
    with open(videos_path, encoding="utf-8") as f:
        videos_file = json.load(f)
    added = films(cards, names, videos_file["videos"], left_out, seen)

    with open(advice_path, "w", encoding="utf-8", newline="\n") as out:
        json.dump(advice, out, ensure_ascii=False, indent=1)
        out.write("\n")
    with open(videos_path, "w", encoding="utf-8", newline="\n") as out:
        json.dump(videos_file, out, ensure_ascii=False, indent=1)
        out.write("\n")
    linked = sum(len(v) for v in advice["links"].values())
    print(f"{len(tips)} safety tips; {linked} links on {len(advice['links'])} cards; "
          f"{added} films added; left out: " + ("; ".join(left_out) if left_out else "nothing"))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
