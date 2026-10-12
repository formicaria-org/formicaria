#!/usr/bin/env python3
"""Fetch the exercise how-tos the app carries: the NHS's and the U.S. Army's, word for word.

    python3 ci/pack-exercises.py crates/fm-app/src/shipped/exercises.json
    python3 ci/pack-exercises.py out.json --atp atp.pdf --pages saved-pages/

**What it is.** A maintainer runs this by hand when the exercise file is wanted afresh. The app
never runs it, and the build reads only the JSON file it leaves behind — which is committed, so
building formicaria needs neither this script, nor `pdftotext`, nor the network.

**Where the exercises are from.** Two public sources.

1. The NHS website, Live Well → Exercise — seven pages under
   https://www.nhs.uk/live-well/exercise/ : `strength-exercises/`, `balance-exercises/`,
   `sitting-exercises/`, `flexibility-exercises/`, `how-to-warm-up-before-exercising/`,
   `how-to-stretch-after-exercising/` and `knee-exercises-for-runners/`. Seven requests, one at a
   time, a second apart.
2. U.S. Army, ATP 7-22.02, "Holistic Health and Fitness Drills and Exercises" (October 2020,
   incorporating Change 1), the official PDF:
   https://armypubs.army.mil/epubs/DR_pubs/DR_a/ARN45013-ATP_7-22.02-001-WEB-4.pdf
   Taken from it: the Preparation Drill, the Hip Stability Drill, the Shoulder Stability Drill,
   Four for the Core, Conditioning Drill 1, Climbing Drill 1, the Strength Training Circuit, Free
   Weight Training (core and assistive), the Strength Training Machine Drill and the Recovery
   Drill — the standard exercises, not the "(MODIFIED)" drills.

**Licence.** Each in the source's own words.

  - NHS: "Information from the NHS website is licensed under the Open Government Licence v3.0"
    (https://www.nationalarchives.gov.uk/doc/open-government-licence/version/3/). That covers the
    words; the NHS's pictures and videos are not reusable and none is taken.
  - Army: the cover of the publication reads "DISTRIBUTION RESTRICTION: Approved for public
    release; distribution unlimited". The script reads that line out of the PDF and writes it as
    the source's `licence`. Its photographs are not taken, only its text.

**What was changed on the way: nothing but whitespace.** Every `name`, `about`, `start`, step,
`dose` and note is the source's own text. HTML entities are decoded, runs of whitespace become
one space, the ends are trimmed. The PDF's wrapped lines are joined again with one space — or
with none where a line ends in a hyphen or a dash that belongs to the word ("90-" / "degree",
"Straight-" / "Arm Pull", "locked—" / "and"); the hyphen is kept, no word is re-spelled. Nothing
is reworded, summarised, reordered within a field, corrected or translated: the sources' slips
are kept ("if possible .", a bullet printed twice, "On count 4" at the end of a stretch that is
done to commands), and so are the Army's commands, cadences and "Soldiers".

**What is not taken.** NHS: pictures, their credits and alt text, videos, the "More in" links,
a lead-in that only announces a list ("To do a buttock stretch:"), and "Neck rotation" on the
sitting page (the flexibility page has it too, and that one is kept). Army: figures and their
captions, page headers and footers, the "(MODIFIED)" drills, and in the Strength Training
Machine Drill the "Modified", "Single-Arm", "Single-Leg" and "Straight-Arm" variants.

**Where each piece of the file comes from.**

  - `name` — NHS: the page's heading, up to its " – " or ": ". Army: the heading, in the capitals
    the paragraph under it prints ("BEND AND REACH" is "Bend and Reach" in its paragraph).
  - `about` — NHS: the paragraph(s) between the heading and step A. Army: the whole numbered
    paragraph above the bullets, figure references and cadence included.
  - `start` — Army only: the first bullet that begins "The starting position for".
  - `steps` — NHS: the "A.", "B.", "C." paragraphs without the letter, or the numbered list, or
    the captions under the pictures (the knee page), or the one paragraph of a warm-up move.
    Army: the remaining bullets, in order.
  - `dose` — NHS: the heading's part after " – " or ": ", else the first paragraph after the
    steps (whole, even when it says more than a number). Army: the LAST bullet, when it is a
    repetitions line; one that stands earlier in the list stays where it is, as a step.
  - `notes` — NHS: later paragraphs after the steps, "Tip: …". Army: bullets about the spotter,
    a boxed CAUTION (its heading, then its points), a "requires a spotter" paragraph and its
    bullets.
  - `equipment_said` — the first sentence, start or step of the entry (for the NHS, failing
    that, of its page's opening paragraphs) that names the equipment. Whole, never trimmed.
  - `equipment` and `area` are the ONLY fields that are this script's and not the source's: two
    hand-made tables below (`NHS_CLASS`, `ARMY`). An exercise with no row there is left out and
    named in the summary line.

**Nothing is invented.** A field the source does not have is not in the file.

The file is written sorted by id, so the same sources make the same file apart from `seen` and
`as_at`, the day this was run. Fewer than 30 NHS or fewer than 50 Army exercises is treated as a
source having changed under the script: nothing is written and the exit status is 1. An Army
section whose text does not parse cleanly is left out whole and named, not guessed at.

Options. `--atp <file>`: a local copy of the PDF (default: download it once to a temporary
file). `--pages <dir>`: keep the NHS pages there — a page already in the folder is read from it
and not requested again, so a second run costs the NHS nothing.

Standard library only — except that the Army part shells out to `pdftotext -layout` (Poppler).
Without `pdftotext` on PATH the script stops with a message; it does not try another way.
"""
import html.parser
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request

USER_AGENT = "formicaria-exercise-pack (https://github.com/singhbal-baljinder/formicaria)"
PAUSE_S = 1.0  # between any two requests
RETRY_PAUSE_S = 5.0
MIN_NHS, MIN_ARMY = 30, 50

NHS_HOME = "https://www.nhs.uk/live-well/exercise/"
NHS_PAGES = [
    "strength-exercises",
    "balance-exercises",
    "sitting-exercises",
    "flexibility-exercises",
    "how-to-warm-up-before-exercising",
    "how-to-stretch-after-exercising",
    "knee-exercises-for-runners",
]
# The same exercise on two pages: the one left out, and why.
NHS_DROP = {("sitting-exercises", "Neck rotation"): "also on flexibility-exercises, kept there"}
ATP_URL = "https://armypubs.army.mil/epubs/DR_pubs/DR_a/ARN45013-ATP_7-22.02-001-WEB-4.pdf"

# ---- The two hand-made columns: (equipment, area). Everything else is the source's. ----------

NHS_CLASS = {
    "strength-exercises": {
        "Sit-to-stand": ("chair or wall", "lower-push"),
        "Mini-squats": ("chair or wall", "lower-push"),
        "Calf raises": ("chair or wall", "lower-push"),
        "Sideways leg lift": ("chair or wall", "lower-pull"),
        "Leg extension": ("chair or wall", "lower-pull"),
        "Wall press-up": ("chair or wall", "upper-push"),
        "Biceps curls": ("free weights", "upper-pull"),
    },
    "balance-exercises": {
        "Sideways walking": ("none", "balance"),
        "Simple grapevine": ("none", "balance"),
        "Heel-to-toe walk": ("none", "balance"),
        "One-leg stand": ("chair or wall", "balance"),
        "Step-up": ("other", "balance"),
    },
    "sitting-exercises": {
        "Chest stretch": ("chair or wall", "flexibility"),
        "Upper-body twist": ("chair or wall", "flexibility"),
        "Hip marching": ("chair or wall", "lower-push"),
        "Ankle stretch": ("chair or wall", "flexibility"),
        "Neck rotation": ("chair or wall", "flexibility"),
    },
    "flexibility-exercises": {
        "Neck rotation": ("none", "flexibility"),
        "Neck stretch": ("none", "flexibility"),
        "Sideways bend": ("none", "flexibility"),
        "Calf stretch": ("chair or wall", "flexibility"),
    },
    "how-to-warm-up-before-exercising": {
        "March on the spot": ("none", "warm-up"),
        "Heel digs": ("none", "warm-up"),
        "Knee lifts": ("none", "warm-up"),
        "Shoulder rolls": ("none", "warm-up"),
        "Knee bends": ("none", "warm-up"),
    },
    "how-to-stretch-after-exercising": {
        "Buttock stretch": ("none", "flexibility"),
        "Hamstring stretch": ("none", "flexibility"),
        "Inner thigh stretch": ("none", "flexibility"),
        "Calf stretch": ("none", "flexibility"),
        "Thigh stretch": ("none", "flexibility"),
    },
    "knee-exercises-for-runners": {
        "Wall squats": ("chair or wall", "lower-push"),
        "Thigh contraction": ("chair or wall", "lower-push"),
        "Hamstring stretch with thigh contraction": ("chair or wall", "flexibility"),
        "Squats": ("none", "lower-push"),
        "Lunges": ("none", "lower-push"),
    },
}

# One row per section of the publication: the group as the text prints it, the heading that
# opens the section, the heading that closes it, headings left out, and the exercises.
STM_VARIANT = r"MODIFIED |SINGLE-ARM |SINGLE-LEG |STRAIGHT-ARM "
ARMY = [
    ("Preparation Drill", "THE PREPARATION DRILL", "PREPARATION DRILL (MODIFIED) (PD MOD)", None, {
        "Bend and Reach": ("none", "warm-up"),
        "Rear Lunge": ("none", "lower-push"),
        "High Jumper": ("none", "whole-body"),
        "Rower": ("none", "trunk"),
        "Squat Bender": ("none", "lower-push"),
        "Windmill": ("none", "trunk"),
        "Forward Lunge": ("none", "lower-push"),
        "Prone Row": ("none", "upper-pull"),
        "Bent-Leg Body Twist": ("none", "trunk"),
        "Push-Up": ("none", "upper-push"),
    }),
    ("Hip Stability Drill", "HIP STABILITY DRILL (HSD)", "SHOULDER STABILITY DRILL (SSD)", None, {
        "Lateral Leg Raise": ("none", "lower-pull"),
        "Medial Leg Raise": ("none", "lower-pull"),
        "Bent-Leg Lateral Raise": ("none", "lower-pull"),
        "Single-Leg Tuck": ("none", "lower-push"),
        "Single Leg Over": ("none", "flexibility"),
    }),
    ("Shoulder Stability Drill", "SHOULDER STABILITY DRILL (SSD)", "FOUR FOR THE CORE (4C)", None, {
        "I Raise": ("none", "upper-pull"),
        "T Raise": ("none", "upper-pull"),
        "Y Raise": ("none", "upper-pull"),
        "L Raise": ("none", "upper-pull"),
        "W Raise": ("none", "upper-pull"),
    }),
    ("Four for the Core", "FOUR FOR THE CORE (4C)", "CONDITIONING DRILL 1 (CD1)", None, {
        "Bent-Leg Raise": ("none", "trunk"),
        "Side Bridge": ("none", "trunk"),
        "Back Bridge": ("none", "trunk"),
        "Quadraplex": ("none", "trunk"),
    }),
    ("Conditioning Drill 1", "CONDITIONING DRILL 1 (CD1)",
     "CONDITIONING DRILL 1 (MODIFIED) (CD1 MOD)", None, {
        "Power Jump": ("none", "whole-body"),
        "V-Up": ("none", "trunk"),
        "Mountain Climber": ("none", "whole-body"),
        "Leg-Tuck and Twist": ("none", "trunk"),
        "Single-Leg Push-Up": ("none", "upper-push"),
    }),
    ("Climbing Drill 1", "CLIMBING DRILL 1 (CL1)", "CLIMBING DRILL 2 (CL2)", None, {
        "Straight-Arm Pull": ("bar", "upper-pull"),
        "Heel Hook": ("bar", "upper-pull"),
        "Pull-Up": ("bar", "upper-pull"),
        "Leg Tuck": ("bar", "trunk"),
        "Alternating Grip Pull-Up": ("bar", "upper-pull"),
    }),
    ("Strength Training Circuit", "STRENGTH TRAINING", "FREE WEIGHTS", None, {
        "Sumo Squat": ("free weights", "lower-push"),
        "Straight-Leg Deadlift": ("free weights", "lower-pull"),
        "Forward Lunge": ("free weights", "lower-push"),
        "8-Count Step-Up": ("free weights", "lower-push"),
        "Pull-Up": ("bar", "upper-pull"),
        "Straight-Arm Pull": ("bar", "upper-pull"),
        "Supine Chest Press": ("free weights", "upper-push"),
        "Bent-Over Row": ("free weights", "upper-pull"),
        "Overhead Push-Press": ("free weights", "upper-push"),
        "Supine Body Twist": ("free weights", "trunk"),
        "Leg Tuck": ("bar", "trunk"),
    }),
    ("Free Weight Core training", "FREE WEIGHT CORE TRAINING EXERCISES",
     "FREE WEIGHT ASSISTIVE TRAINING EXERCISES", None, {
        "Front Squat": ("free weights", "lower-push"),
        "Back Squat": ("free weights", "lower-push"),
        "Deadlift": ("free weights", "lower-pull"),
        "Straight-Leg Deadlift": ("free weights", "lower-pull"),
        "Bench Press": ("free weights", "upper-push"),
        "Incline Bench": ("free weights", "upper-push"),
    }),
    ("Free Weight Assistive training", "FREE WEIGHT ASSISTIVE TRAINING EXERCISES",
     "STRENGTH TRAINING MACHINES", None, {
        "Sumo Deadlift": ("free weights", "lower-pull"),
        "Heel Raise": ("free weights", "lower-push"),
        "Bent-Over Row": ("free weights", "upper-pull"),
        "Single-Arm Bent-Over Row": ("free weights", "upper-pull"),
        "Upright Row": ("free weights", "upper-pull"),
        "Overhead Push-Press": ("free weights", "upper-push"),
        "Bent-Arm Lateral Raise": ("free weights", "upper-push"),
        "Shrug": ("free weights", "upper-pull"),
        "Pull Over": ("free weights", "upper-push"),
        "Overhead Triceps Extension": ("free weights", "upper-push"),
        "Biceps Curl": ("free weights", "upper-pull"),
        "Weighted Trunk Flexion": ("free weights", "trunk"),
        "Weighted Trunk Extension": ("free weights", "trunk"),
    }),
    ("Strength Training Machine Drill", "STRENGTH TRAINING MACHINES", "THE RECOVERY DRILL",
     STM_VARIANT, {
        "Leg Press": ("machine", "lower-push"),
        "Leg Curl": ("machine", "lower-pull"),
        "Lateral Raise": ("machine", "upper-push"),
        "Overhead Press": ("machine", "upper-push"),
        "Lat Pull-Down": ("machine", "upper-pull"),
        "Seated Row": ("machine", "upper-pull"),
        "Trunk Extension": ("machine", "trunk"),
        "Triceps Extension": ("machine", "upper-push"),
        "Chest Press": ("machine", "upper-push"),
        "Trunk Flexion": ("machine", "trunk"),
    }),
    ("Recovery Drill", "THE RECOVERY DRILL", "THE RECOVERY DRILL (MODIFIED)", None, {
        "Overhead Arm Pull": ("none", "flexibility"),
        "Rear Lunge": ("none", "flexibility"),
        "Extend and Flex": ("none", "flexibility"),
        "Thigh Stretch": ("none", "flexibility"),
        "Single-Leg Over": ("none", "flexibility"),
        "Groin Stretch": ("none", "flexibility"),
        "Calf Stretch": ("none", "flexibility"),
        "Hamstring Stretch": ("none", "flexibility"),
    }),
]

# The words that show a sentence is about the equipment, per `equipment` value.
SAID = {
    "chair or wall": r"\b(?:chairs?|wall)\b",
    "bar": r"\bbar\b",
    "free weights": r"\b(?:kettlebells?|dumbbells?|bar|barbell|light weights|plate)\b",
    "machine": r"\b(?:weight stack|machine(?! drill)|handgrips|foot platform|seat back|padded lever)\b",
    "other": r"\b(?:Use a step|-inch step)\b",
}

_last_request = [0.0]


def fetch(url, binary=False):
    """The page (or file): one request at a time, a pause before each, one retry."""
    for attempt in (1, 2):
        wait = PAUSE_S - (time.monotonic() - _last_request[0])
        if wait > 0:
            time.sleep(wait)
        request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
        try:
            with urllib.request.urlopen(request, timeout=120) as response:
                data = response.read()
                return data if binary else data.decode("utf-8")
        except (urllib.error.URLError, OSError, UnicodeDecodeError) as error:
            if attempt == 2:
                raise RuntimeError(f"{url}: {error}") from error
            time.sleep(RETRY_PAUSE_S)
        finally:
            _last_request[0] = time.monotonic()


def tidy(text):
    """Whitespace only: runs (non-breaking spaces included) to one space, ends trimmed."""
    return re.sub(r"\s+", " ", text).strip()


def slug(text):
    return re.sub(r"[^a-z0-9]+", "-", text.lower()).strip("-")


def sentences(text):
    return re.split(r"(?<=[.:])\s+(?=[A-Z])", text)


def said(entry, extra=()):
    """The first sentence / start / step that names the equipment, whole — or None."""
    pattern = SAID.get(entry["equipment"])
    if not pattern:
        return None
    pieces = sentences(entry.get("about", "")) + [entry.get("start", "")] + entry["steps"]
    pieces += [entry.get("dose", "")] + list(extra)
    for piece in pieces:
        if piece and re.search(pattern, piece):
            return piece
    return None


def ordered(entry):
    keys = ["id", "source", "name", "url", "ref", "group", "equipment", "equipment_said", "area",
            "about", "start", "steps", "dose", "notes"]
    return {key: entry[key] for key in keys if entry.get(key)}


# ---- NHS -------------------------------------------------------------------------------------


class NhsPage(html.parser.HTMLParser):
    """Reads <main id="maincontent"> into blocks: (kind, text, letter). Pictures are skipped."""

    SKIP = {"aside", "nav", "details", "script", "style", "svg", "noscript"}

    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.blocks = []
        self._main = False
        self._skip = 0  # depth inside an element whose text is not content
        self._figure = 0
        self._ordered = []  # the open lists: True for <ol>
        self._kind = None  # the element being read: h1 | h2 | p | li | figcaption
        self._text = []
        self._lead = None  # the bold text a <p> opens with
        self._bold = False

    def _flush(self):
        if self._kind:
            text = tidy("".join(self._text))
            letter = None
            if self._kind == "p" and self._lead is not None:
                lead = tidy(self._lead)
                if re.fullmatch(r"[A-Z]\.", lead) and text.startswith(lead):
                    letter, text = lead[0], text[len(lead):].strip()
            kind = self._kind
            if kind == "li":
                kind = "step" if self._ordered and self._ordered[-1] else "item"
            if text:
                self.blocks.append((kind, text, letter))
        self._kind, self._text, self._lead, self._bold = None, [], None, False

    def handle_starttag(self, tag, attrs):
        if tag == "main":
            self._main = dict(attrs).get("id") == "maincontent"
            return
        if not self._main:
            return
        if tag in self.SKIP:
            self._flush()
            self._skip += 1
        if self._skip:
            return
        if tag == "figure":
            self._flush()
            self._figure += 1
        elif tag in ("ol", "ul"):
            self._flush()
            self._ordered.append(tag == "ol")
        elif tag in ("h1", "h2", "h3", "li", "figcaption") or (tag == "p" and not self._figure):
            self._flush()
            self._kind = tag
        elif tag == "p" and self._figure and self._kind != "figcaption":
            self._flush()  # a picture's credit: not content
        elif tag in ("b", "strong") and self._kind == "p" and not tidy("".join(self._text)):
            self._bold = True
        elif tag == "br" and self._kind:
            self._text.append(" ")

    def handle_endtag(self, tag):
        if tag == "main":
            self._flush()
            self._main = False
            return
        if not self._main:
            return
        if tag in self.SKIP and self._skip:
            self._skip -= 1
            return
        if self._skip:
            return
        if tag == "figure" and self._figure:
            self._flush()
            self._figure -= 1
        elif tag in ("ol", "ul"):
            self._flush()
            if self._ordered:
                self._ordered.pop()
        elif tag in ("b", "strong") and self._bold:
            self._bold = False
            self._lead = "".join(self._text)
        elif tag == self._kind or tag in ("section", "article", "div") and self._kind == "p":
            self._flush()

    def handle_data(self, data):
        if self._main and not self._skip and self._kind:
            if self._kind == "figcaption" or not self._figure:
                self._text.append(data)


def nhs_page(page, text):
    """One page: its record for the file, its exercises, and what was left out."""
    parser = NhsPage()
    parser.feed(text)
    parser.close()
    url = NHS_HOME + page + "/"
    title, reviewed, notes, sections, skipped = None, None, [], [], []
    for kind, body, letter in parser.blocks:
        if kind == "h1" and title is None:
            title = body
        elif kind == "h2":
            if body.startswith("More in"):
                break
            sections.append((body, []))
        elif kind == "p" and body.startswith("Page last reviewed:"):
            found = re.match(r"Page last reviewed: \d{1,2} \w+ \d{4}", body)
            reviewed = found.group(0) if found else None
        elif sections:
            sections[-1][1].append((kind, body, letter))
        elif kind == "p" and not body.endswith(":"):
            notes.append(body)
    if not title or not reviewed:
        return None, [], [f"{page}: no title or no 'Page last reviewed' line"]

    ref = f"{title} · page last reviewed {reviewed[len('Page last reviewed: '):]}"
    exercises = []
    for heading, blocks in sections:
        split = re.fullmatch(r"(.+?) – (.+)", heading) or re.fullmatch(r"(.+?): (.+)", heading)
        name, dose = (split.group(1), split.group(2)) if split else (heading, None)
        lettered = [i for i, block in enumerate(blocks) if block[2]]
        listed = [body for kind, body, _ in blocks if kind == "step"]
        captions = [body for kind, body, _ in blocks if kind == "figcaption"]
        plain = [body for kind, body, _ in blocks if kind == "p"]
        entry = {"source": "nhs", "name": name, "url": url, "ref": ref, "group": title}
        after = []
        if name == "Video":
            skipped.append(f"{page}: '{heading}' (a video)")
            continue
        if lettered:
            before = [b for k, b, _ in blocks[:lettered[0]] if k == "p"]
            if before:
                entry["about"] = " ".join(before)
            entry["steps"] = [blocks[i][1] for i in lettered]
            after = [b for k, b, _ in blocks[lettered[-1] + 1:] if k == "p"]
            if after and dose is None:
                dose, after = after[0], after[1:]
        elif listed:
            entry["steps"] = listed
            after = [body for body in plain if not body.endswith(":")]
        elif captions:
            entry["steps"] = captions
            after = plain
        elif dose and len(blocks) == 1 and blocks[0][0] == "p":
            entry["steps"] = plain
        else:
            skipped.append(f"{page}: '{heading}' (no steps under it)")
            continue
        if dose:
            entry["dose"] = dose
        if after:
            entry["notes"] = after
        if (page, name) in NHS_DROP:
            skipped.append(f"{page}: '{name}' ({NHS_DROP[(page, name)]})")
            continue
        if name not in NHS_CLASS.get(page, {}):
            skipped.append(f"{page}: '{name}' (no row in NHS_CLASS)")
            continue
        entry["equipment"], entry["area"] = NHS_CLASS[page][name]
        entry["equipment_said"] = said(entry, [s for note in notes for s in sentences(note)])
        entry["_slug"], entry["_group_slug"] = slug(name), page
        exercises.append(entry)
    record = {"url": url, "title": title, "reviewed": reviewed}
    if notes:
        record["notes"] = notes
    return record, exercises, skipped


def nhs(pages_dir):
    records, exercises, skipped = [], [], []
    for page in NHS_PAGES:
        saved = os.path.join(pages_dir, page + ".html") if pages_dir else None
        try:
            if saved and os.path.exists(saved):
                with open(saved, encoding="utf-8") as kept:
                    text = kept.read()
            else:
                text = fetch(NHS_HOME + page + "/")
                if saved:
                    os.makedirs(pages_dir, exist_ok=True)
                    with open(saved, "w", encoding="utf-8", newline="") as keep:
                        keep.write(text)
        except RuntimeError as error:
            skipped.append(f"{page}: could not be fetched ({error})")
            continue
        record, found, left = nhs_page(page, text)
        if record:
            records.append(record)
        exercises += found
        skipped += left
    return records, exercises, skipped


# ---- Army ------------------------------------------------------------------------------------

BULLET = "⚫"  # the publication's list mark, as pdftotext gives it
FOOTER = re.compile(r"ATP 7-22\.02 \(INCL C1\)")
CAPTION = re.compile(r"\s*\+?Figures? \d+-\d+\.\s")
PARAGRAPH = re.compile(r"\s*\+?(\d{1,2}-\d{1,3})\.\s+(\S.*)")
HEADING = re.compile(r"(?:\d{1,2}[AB]?\s?[.:]\s?)?\+?([A-Z0-9][A-Z0-9 ,'’()\-]*[A-Z)])")
START = re.compile(r"(?:The starting|Starting) position for |To reach the starting position ")
DOSE = re.compile(
    r"Perform \d|Repeat \d|Repeat the exercise |Repeat this movement for the correct number "
    r"|Complete one minute |Continue the exercise at own pace "
)
SPOTTER_PARAGRAPH = re.compile(r"requires? a spott")
# What must never be inside a field: a caption, a header or footer, a paragraph number.
STRAY = re.compile(r"Figure \d+-\d+\. |ATP 7-22\.02|\d\d October 20\d\d|This page intentionally"
                   r"|(?:^| )\d{1,2}-\d{1,3}\. [A-Z]|⚫|•")


def join(lines):
    """Wrapped lines back into one: a space between, none after a word's own hyphen or dash."""
    out = ""
    for line in lines:
        line = tidy(line)
        if not line:
            continue
        if not out:
            out = line
        elif re.search(r"\S[-—]$", out):
            out += line
        else:
            out += " " + line
    return out


def atp_text(path):
    if not shutil.which("pdftotext"):
        raise SystemExit("pack-exercises.py: `pdftotext` (Poppler) is not on PATH — the Army part "
                         "needs it; nothing written")
    done = subprocess.run(["pdftotext", "-layout", path, "-"], capture_output=True, check=False)
    if done.returncode != 0:
        raise SystemExit(f"pack-exercises.py: pdftotext failed on {path}: "
                         f"{done.stderr.decode('utf-8', 'replace').strip()}")
    return done.stdout.decode("utf-8")


def atp_lines(text):
    """The publication's lines without page furniture: footers, running headers, captions."""
    pages = [page.split("\n") for page in text.split("\f")]
    titles = set()  # the chapter titles, which are also the even pages' running header
    for lines in pages:
        full = [line for line in lines if line.strip()]
        if len(full) > 1 and re.fullmatch(r"Chapter \d+", full[0].strip()):
            second = full[1]
            if (len(second) - len(second.lstrip()) >= 15 and re.search(r"[a-z]", second)
                    and BULLET not in second and not CAPTION.match(second)):
                titles.add(second.strip().lstrip("+"))
    kept = []
    for lines in pages:
        seen = 0  # non-blank lines so far on this page
        opening = False  # a chapter's first page: "Chapter 14", then its title
        for index, line in enumerate(lines):
            bare = line.strip()
            if not bare:
                continue
            seen += 1
            if seen == 1:
                opening = bool(re.fullmatch(r"Chapter \d+", bare))
                if opening or bare.lstrip("+") in titles:
                    continue
            if seen == 2 and opening and bare.lstrip("+") in titles:
                continue
            if FOOTER.search(line) or bare == "This page intentionally left blank.":
                continue
            if CAPTION.match(line) and (index == 0 or not lines[index - 1].strip()):
                continue
            kept.append(line)
    return kept


def atp_blocks(lines):
    """Lines into blocks: heading | caution | paragraph | bullet | point, each with its text."""
    blocks = []
    for line in lines:
        bare = line.strip()
        numbered = PARAGRAPH.fullmatch(line)
        heading = HEADING.fullmatch(bare)
        if numbered:
            blocks.append({"kind": "paragraph", "number": numbered.group(1),
                           "lines": [numbered.group(2)]})
        elif bare.startswith(BULLET):
            blocks.append({"kind": "bullet", "lines": [bare[len(BULLET):]]})
        elif bare.startswith("•"):
            blocks.append({"kind": "point", "lines": [bare[1:]]})
        elif heading and len(re.findall(r"[A-Z]", bare)) >= 3 and not re.search(r"[a-z]", bare):
            kind = "caution" if re.search(r"CAUTIONS?$", bare) else "heading"
            blocks.append({"kind": kind, "lines": [bare if kind == "caution" else heading.group(1)]})
        elif blocks:
            blocks[-1]["lines"].append(line)
    for block in blocks:
        block["text"] = join(block.pop("lines"))
    return blocks


def printed_name(heading, text):
    """The heading in the capitals the paragraph under it uses, or None."""
    found = [m.group(0) for m in re.finditer(r"(?<![\w-])" + re.escape(heading) + r"(?![\w-])",
                                             text, re.IGNORECASE)]
    proper = [name for name in found if name != name.lower() and name != name.upper()]
    proper += [name for name in found if name != name.lower()]
    return proper[0] if proper else None


def army_section(blocks, group, opener, closer, leave_out, table):
    """One section's exercises, or (None, reason) when its text does not parse cleanly."""
    at = [i for i, b in enumerate(blocks) if b["kind"] == "heading" and b["text"] == opener]
    if len(at) != 1:
        return None, f"its heading '{opener}' was found {len(at)} times"
    end = next((i for i in range(at[0] + 1, len(blocks))
                if blocks[i]["kind"] == "heading" and blocks[i]["text"] == closer), None)
    if end is None:
        return None, f"its closing heading '{closer}' was not found"

    exercises, left_out = [], []
    heading, fresh, current, target = None, False, None, None
    skipping = True  # until the first exercise heading
    for block in blocks[at[0] + 1:end]:
        kind, text = block["kind"], block["text"]
        if kind == "heading":
            heading, fresh, current, target = text, True, None, None
            skipping = bool(leave_out and re.match(leave_out, text))
            if skipping:
                left_out.append(text)
        elif skipping:
            continue
        elif kind == "paragraph":
            target = None
            if SPOTTER_PARAGRAPH.search(text):
                named = [e for e in exercises if re.search(
                    r"(?<![\w-])" + re.escape(e["name"]) + r"(?![\w-])", text.split(". ")[0])]
                if not named:
                    return None, f"paragraph {block['number']}: a spotter paragraph for no exercise"
                target = {"lead": text, "bullets": []}
                for exercise in named:
                    exercise["_spot"].append(target)
                    exercise["_numbers"].append(block["number"])
            elif current is None or current["_bullets"]:
                name = printed_name(heading, text) if fresh else None
                if not fresh:
                    told = re.match(r"The ((?:[A-Z0-9][\w-]*)(?: [A-Z0-9][\w-]*)*) is ", text)
                    name = told.group(1) if told else None
                if not name:
                    return None, f"paragraph {block['number']}: no name for the exercise"
                current = {"name": name, "about": text, "_numbers": [block["number"]],
                           "_bullets": [], "_caution": [], "_spot": []}
                exercises.append(current)
                fresh = False
            else:
                # A paragraph with no bullets, then one with them (the Rower): both are `about`.
                current["about"] += " " + text
                current["_numbers"].append(block["number"])
        elif kind == "bullet":
            if target is not None:
                target["bullets"].append(text)
            elif current is not None:
                current["_bullets"].append(text)
            else:
                return None, f"a bullet under '{heading}' before any paragraph"
        elif kind in ("caution", "point"):
            if current is None:
                return None, f"a caution under '{heading}' before any paragraph"
            current["_caution"].append(text)
    out = []
    for exercise in exercises:
        bullets = exercise.pop("_bullets")
        if not bullets:
            # A heading with a paragraph and no bullets under it: a variant, not a how-to.
            left_out.append(exercise["name"])
            continue
        name = exercise["name"]
        if name not in table:
            return None, f"'{name}' has no row in ARMY"
        notes = [b for b in bullets if re.search(r"spott", b, re.IGNORECASE)]
        rest = [b for b in bullets if b not in notes]
        start = next((b for b in rest if START.match(b)), None)
        steps = [b for b in rest if b is not start]
        dose = None
        if steps and DOSE.match(steps[-1]):
            dose, steps = steps[-1], steps[:-1]
        if not steps:
            return None, f"'{name}' has no steps"
        notes += exercise.pop("_caution")
        for spot in exercise.pop("_spot"):
            notes += [spot["lead"]] + spot["bullets"]
        numbers = exercise.pop("_numbers")
        label = "paragraph " if len(numbers) == 1 else "paragraphs "
        entry = {"source": "army", "name": name, "url": ATP_URL,
                 "ref": f"{label}{', '.join(numbers)} · {group}", "group": group,
                 "about": exercise["about"], "steps": steps}
        if start:
            entry["start"] = start
        if dose:
            entry["dose"] = dose
        if notes:
            entry["notes"] = notes
        entry["equipment"], entry["area"] = table[name]
        entry["equipment_said"] = said(entry)
        entry["_slug"], entry["_group_slug"] = slug(name), slug(group)
        fields = [entry["about"], entry.get("start", ""), entry.get("dose", "")] + steps + notes
        stray = next((STRAY.search(f) for f in fields if STRAY.search(f)), None)
        if stray:
            return None, f"'{name}' carries page furniture ({stray.group(0)!r})"
        out.append(entry)
    missing = [name for name in table if name not in [e["name"] for e in out]]
    if missing:
        return None, f"not found: {', '.join(missing)}"
    return (out, left_out), None


def army(path):
    text = atp_text(path)
    licence = next((tidy(line) for line in text.split("\n")
                    if line.strip().startswith("DISTRIBUTION RESTRICTION:")), None)
    change = next((tidy(line) for line in text.split("\n")
                   if re.match(r"\s*\d+\. ATP 7-22\.02, .* is changed as follows:", line)), None)
    blocks = atp_blocks(atp_lines(text))
    if text and not any(b["kind"] == "bullet" for b in blocks):
        raise SystemExit("pack-exercises.py: no list marks in the PDF's text — a different "
                         "pdftotext, or a different PDF; nothing written")
    exercises, skipped = [], []
    for group, opener, closer, leave_out, table in ARMY:
        found, reason = army_section(blocks, group, opener, closer, leave_out, table)
        if found is None:
            skipped.append(f"{group}: left out whole — {reason}")
            continue
        exercises += found[0]
        if found[1]:
            skipped.append(f"{group}: variants not taken ({', '.join(found[1])})")
    return licence, change, exercises, skipped


# ---- The file --------------------------------------------------------------------------------


def with_ids(source, exercises):
    """`<source>-<slug>`; a name that repeats within the source gets its group's slug too."""
    count = {}
    for entry in exercises:
        count[entry["_slug"]] = count.get(entry["_slug"], 0) + 1
    for entry in exercises:
        entry["id"] = f"{source}-{entry['_slug']}"
        if count[entry["_slug"]] > 1:
            entry["id"] += "-" + entry["_group_slug"]
    return exercises


def main(argv):
    args, atp, pages_dir = [], None, None
    rest = argv[1:]
    while rest:
        arg = rest.pop(0)
        if arg in ("--atp", "--pages") and rest:
            if arg == "--atp":
                atp = rest.pop(0)
            else:
                pages_dir = rest.pop(0)
        else:
            args.append(arg)
    if len(args) != 1 or args[0].startswith("--"):
        print("usage: pack-exercises.py <output.json> [--atp <ATP 7-22.02 pdf>] [--pages <dir>]",
              file=sys.stderr)
        return 2

    records, from_nhs, skipped = nhs(pages_dir)
    downloaded = None
    try:
        if atp is None:
            handle, downloaded = tempfile.mkstemp(suffix=".pdf")
            with os.fdopen(handle, "wb") as out:
                out.write(fetch(ATP_URL, binary=True))
            atp = downloaded
        licence, change, from_army, left = army(atp)
    finally:
        if downloaded and os.path.exists(downloaded):
            os.remove(downloaded)
    skipped += left

    exercises = with_ids("nhs", from_nhs) + with_ids("army", from_army)
    exercises.sort(key=lambda entry: entry["id"])
    ids = [entry["id"] for entry in exercises]
    summary = (f"{len(from_nhs)} NHS exercises from {len(records)} pages, {len(from_army)} Army "
               f"exercises; left out: " + ("; ".join(skipped) if skipped else "nothing"))
    problem = None
    if len(from_nhs) < MIN_NHS:
        problem = f"fewer than {MIN_NHS} NHS exercises"
    elif len(from_army) < MIN_ARMY:
        problem = f"fewer than {MIN_ARMY} Army exercises"
    elif len(set(ids)) != len(ids):
        problem = "two exercises with one id"
    elif not licence:
        problem = "no distribution statement on the Army cover"
    if problem:
        print(f"{summary} — {problem}, nothing written", file=sys.stderr)
        return 1

    army_source = {
        "id": "army",
        "by": "U.S. Army",
        "credit": "Source: U.S. Army, ATP 7-22.02, Holistic Health and Fitness Drills and "
                  "Exercises (October 2020, including Change 1)",
        "licence": licence,
        "home": ATP_URL,
    }
    if change:
        army_source["change"] = change
    pack = {
        "seen": time.strftime("%Y-%m-%d"),
        "sources": [
            {
                "id": "nhs",
                "by": "NHS website",
                "credit": "Information from the NHS website",
                "as_at": time.strftime("%d%m%y"),
                "licence": "Information from the NHS website is licensed under the Open "
                           "Government Licence v3.0",
                "licence_url":
                    "https://www.nationalarchives.gov.uk/doc/open-government-licence/version/3/",
                "home": NHS_HOME,
                "pages": records,
            },
            army_source,
        ],
        "exercises": [ordered(entry) for entry in exercises],
    }
    with open(args[0], "w", encoding="utf-8", newline="\n") as out:
        json.dump(pack, out, ensure_ascii=False, indent=1)
        out.write("\n")
    print(summary)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
