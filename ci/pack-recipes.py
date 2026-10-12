#!/usr/bin/env python3
"""Fetch the recipes the app carries: the NHLBI's "Healthy Eating Recipes", word for word.

    python3 ci/pack-recipes.py crates/fm-app/src/shipped/recipes-nhlbi.json

**What it is.** A maintainer runs this by hand when the recipe file is wanted afresh. The app
never runs it, and the build reads only the JSON file it leaves behind — which is committed, so
building formicaria needs neither this script nor the network.

**Where the recipes are from.** The U.S. National Heart, Lung, and Blood Institute:

    https://www.nhlbi.nih.gov/health/heart-healthy-living/healthy-foods/healthy-eating-recipes

The listing is paged, ten to a page (`?page=0`, `?page=1`, …); every recipe has a page of its own
with its ingredients, its directions, a "Yields" and a "Serving Size" line and a table of
nutrition per serving.

**Licence.** The NHLBI's own words, read 2026-10-12 at
https://www.nhlbi.nih.gov/about/contact/trademark-branding-and-logo : "Most of the information on
the NHLBI website is in the public domain and can be used without restriction. The NHLBI asks
only that no changes be made to the publications, videos, images, or other formatted multimedia
products and that the material, as well any NHLBI webpage links, not be used in any direct or
indirect product endorsement or advertising." It asks to be cited as "Source: National Heart,
Lung, and Blood Institute; National Institutes of Health; U.S. Department of Health and Human
Services", which is the file's `source`. It adds that "copyrighted materials will include a
copyright statement" — so a recipe page carrying one (or crediting someone's permission) is left
out here and named in the summary line.

**What was changed on the way: nothing but whitespace.** Each ingredient line and each step is
the page's own text. HTML entities are decoded, runs of whitespace (the page's non-breaking
spaces among them) become one space, and the ends are trimmed. Nothing is reworded, reordered,
corrected or translated — the page's typing slips are kept too. A sub-heading among the
ingredients ("For the sauce:") is a line of its own, where the page has it.

**What is not taken.** The opening sentence, the "Recipe Source" line, the tips and notes under
the directions, preparation and cooking times, the video, and the nutrients the file has no key
for (calcium, iron, vitamins A and C, magnesium).

**Nothing is invented.** A field the page does not have is not in the file.

  - `yield` is the page's "Yields" text, then "; Serving Size: " and the page's "Serving Size"
    text — both verbatim, joined by the page's own label.
  - `servings` is there only when "Yields" reads exactly "<number> servings".
  - `per_serving` holds whole numbers in the unit each key names, changed only by scale: g to mg
    and mg to µg are ×1000 (and g to µg ×1,000,000, where a page states a µg-keyed nutrient in
    grams). A value that is not a plain number and a unit — "0 (less than 1 g)" — has no key.

The file is written sorted by id, so the same pages make the same file apart from `seen`, the
day this was run. Fewer than 40 recipes is treated as the site having changed under the script:
nothing is written and the exit status is 1.

Standard library only.
"""
import html.parser
import html
import json
import re
import sys
import time
import urllib.error
import urllib.request

SOURCE = (
    "National Heart, Lung, and Blood Institute; National Institutes of Health; "
    "U.S. Department of Health and Human Services"
)
SITE = "https://www.nhlbi.nih.gov"
HOME_PATH = "/health/heart-healthy-living/healthy-foods/healthy-eating-recipes"
HOME = SITE + HOME_PATH

USER_AGENT = "formicaria-recipe-pack (https://github.com/singhbal-baljinder/formicaria)"
PAUSE_S = 1.0  # between any two requests
RETRY_PAUSE_S = 5.0
MAX_LISTING_PAGES = 30  # a stop for a listing that never runs out
MIN_RECIPES = 40

# The page's nutrition field -> (the file's key, the unit the key names).
NUTRIENTS = {
    "total-fat": ("fat_mg", "mg"),
    "saturated-fat": ("saturated_fat_mg", "mg"),
    "cholesterol": ("cholesterol_ug", "ug"),
    "sodium": ("sodium_ug", "ug"),
    "total-fiber": ("fiber_mg", "mg"),
    "protein": ("protein_mg", "mg"),
    "carbohydrates": ("carbs_mg", "mg"),
    "potassium": ("potassium_ug", "ug"),
}
KEY_ORDER = ["kcal"] + [key for key, _ in NUTRIENTS.values()]
# How many of the key's unit make one of the page's unit.
SCALE = {("g", "mg"): 1000, ("mg", "ug"): 1000, ("g", "ug"): 1000000}

# Words that mean a page is not, or not only, the NHLBI's to give away.
NOT_PUBLIC = re.compile(
    r"©|\(c\)\s*\d{4}|copyright|all rights reserved|with permission|by permission"
    r"|courtesy of|not in the public domain",
    re.IGNORECASE,
)

_last_request = [0.0]


def fetch(url):
    """The page as text: one request at a time, a pause before each, one retry."""
    for attempt in (1, 2):
        wait = PAUSE_S - (time.monotonic() - _last_request[0])
        if wait > 0:
            time.sleep(wait)
        request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                return response.read().decode("utf-8")
        except (urllib.error.URLError, OSError, UnicodeDecodeError) as error:
            if attempt == 2:
                raise RuntimeError(f"{url}: {error}") from error
            time.sleep(RETRY_PAUSE_S)
        finally:
            _last_request[0] = time.monotonic()


def tidy(text):
    """Whitespace only: runs (non-breaking spaces included) to one space, ends trimmed."""
    return re.sub(r"\s+", " ", text).strip()


def listing():
    """Every recipe path in listing order, and the count the site says it has (or None)."""
    link = re.compile(r'href="(' + re.escape(HOME_PATH) + r'/[^"/?#]+)"')
    paths, stated = [], None
    for page in range(MAX_LISTING_PAGES):
        text = fetch(f"{HOME}?page={page}")
        said = re.search(r"out of (\d+)</b>\s*results", text)
        if said:
            stated = int(said.group(1))
        new = [path for path in dict.fromkeys(link.findall(text)) if path not in paths]
        if not new:
            break
        paths += new
    return paths, stated


class RecipePage(html.parser.HTMLParser):
    """Reads one recipe page's <article>: title, ingredient lines, steps, the side tables."""

    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.in_article = False
        self.article_text = []
        self.title = None
        self.ingredients = []
        self.steps = []
        self.fields = {}
        self._h1 = None  # text of the <h1> being read
        self._h2 = None  # text of the <h2> being read
        self._body_depth = 0  # > 0 inside the body field's <div>
        self._section = None  # "ingredients" | "steps" | None
        self._line = None  # text of the <li> or <p> being read
        self._field = None  # [name, depth, text] of the side-table field being read

    def _flush(self):
        if self._line is not None:
            text = tidy("".join(self._line))
            if text and self._section == "ingredients":
                self.ingredients.append(text)
            elif text and self._section == "steps":
                self.steps.append(text)
        self._line = None

    def handle_starttag(self, tag, attrs):
        classes = (dict(attrs).get("class") or "").split()
        if tag == "article":
            self.in_article = "node--type-recipe" in classes
            return
        if not self.in_article:
            return
        if tag == "h1" and self.title is None:
            self._h1 = []
        if tag == "div":
            if self._body_depth:
                self._body_depth += 1
            elif "field--name-body" in classes:
                self._body_depth = 1
            if self._field:
                self._field[1] += 1
            else:
                for name in classes:
                    if name.startswith("field--name-field-"):
                        self._field = [name[len("field--name-field-"):], 1, []]
        if not self._body_depth:
            return
        if tag == "h2":
            self._flush()
            self._h2 = []
        elif tag in ("ul", "ol"):
            # A list opening inside a line ends that line: "For garnish:" is its own line.
            self._flush()
        elif tag == "li" or (tag == "p" and self._section == "ingredients"):
            self._flush()
            if self._section:
                self._line = []

    def handle_endtag(self, tag):
        if tag == "article":
            self.in_article = False
            return
        if not self.in_article:
            return
        if tag == "h1" and self._h1 is not None:
            self.title = tidy("".join(self._h1))
            self._h1 = None
        if tag == "div":
            if self._field:
                self._field[1] -= 1
                if self._field[1] == 0:
                    self.fields[self._field[0]] = tidy("".join(self._field[2]))
                    self._field = None
            if self._body_depth:
                self._body_depth -= 1
                if not self._body_depth:
                    self._flush()
                    self._section = None
        if not self._body_depth:
            return
        if tag == "h2" and self._h2 is not None:
            heading = tidy("".join(self._h2))
            self._section = {"Ingredients": "ingredients", "Directions": "steps"}.get(heading)
            self._h2 = None
        elif tag in ("li", "p"):
            self._flush()

    def handle_data(self, data):
        if not self.in_article:
            return
        self.article_text.append(data)
        if self._h1 is not None:
            self._h1.append(data)
        if self._h2 is not None:
            self._h2.append(data)
        elif self._line is not None:
            self._line.append(data)
        if self._field:
            self._field[2].append(data)


def amount(text, key_unit):
    """"5 g" as a whole number of the key's unit, or None for anything but number-and-unit."""
    stated = re.fullmatch(r"(\d{1,3}(?:,\d{3})+|\d+)(?:\.(\d+))? ?(g|mg)", text)
    if not stated:
        return None
    whole, decimals, unit = stated.group(1).replace(",", ""), stated.group(2) or "", stated.group(3)
    scale = SCALE.get((unit, key_unit))
    if scale is None:
        return None
    scaled = int(whole + decimals) * scale
    if scaled % (10 ** len(decimals)):
        return None
    return scaled // (10 ** len(decimals))


def recipe(path, text):
    """The file's entry for one page, or (None, reason) when the page is left out."""
    page = RecipePage()
    page.feed(text)
    page.close()

    noticed = NOT_PUBLIC.search("".join(page.article_text))
    if noticed:
        return None, f"carries a notice ({noticed.group(0)!r})"
    if not page.title:
        return None, "no title"
    if len(page.ingredients) < 2:
        return None, "fewer than 2 ingredients found"
    if not page.steps:
        return None, "no steps found"

    entry = {"id": "nhlbi-" + path.rsplit("/", 1)[1], "title": page.title, "url": SITE + path}

    yields, size = page.fields.get("yields"), page.fields.get("serving-size")
    if yields and size:
        entry["yield"] = f"{yields}; Serving Size: {size}"
    elif yields:
        entry["yield"] = yields
    elif size:
        entry["yield"] = f"Serving Size: {size}"
    servings = re.fullmatch(r"(\d+) servings", yields or "")
    if servings:
        entry["servings"] = int(servings.group(1))

    entry["ingredients"] = page.ingredients
    entry["steps"] = page.steps

    # What the page prints under its directions — a tip, a note, a variation — is part of the
    # recipe as published (some are about safety), so it is carried too, each paragraph whole.
    after = re.search(r"Directions</h2>\s*<ol>.*?</ol>(.*?)</div>", text, re.S)
    notes = [
        tidy(html.unescape(re.sub(r"<[^>]+>", "", p)))
        for p in re.findall(r"<p[^>]*>(.*?)</p>", after.group(1) if after else "", re.S)
    ]
    notes = [n for n in notes if n]
    if notes:
        entry["notes"] = notes

    per_serving = {}
    if re.fullmatch(r"\d+", page.fields.get("calories", "")):
        per_serving["kcal"] = int(page.fields["calories"])
    for field, (key, key_unit) in NUTRIENTS.items():
        value = amount(page.fields.get(field, ""), key_unit)
        if value is not None:
            per_serving[key] = value
    if per_serving:
        entry["per_serving"] = {key: per_serving[key] for key in KEY_ORDER if key in per_serving}
    return entry, None


def main(argv):
    if len(argv) != 2:
        print("usage: pack-recipes.py <output.json>", file=sys.stderr)
        return 2
    paths, stated = listing()
    recipes, skipped = [], []
    for path in paths:
        slug = path.rsplit("/", 1)[1]
        try:
            entry, reason = recipe(path, fetch(SITE + path))
        except RuntimeError as error:
            entry, reason = None, f"could not be fetched ({error})"
        if entry:
            recipes.append(entry)
        else:
            skipped.append(f"{slug}: {reason}")
    recipes.sort(key=lambda entry: entry["id"])

    summary = f"{len(recipes)} recipes from {len(paths)} listed"
    if stated is not None:
        summary += f" (the site says {stated})"
    summary += "; skipped: " + ("; ".join(skipped) if skipped else "none")
    if len(recipes) < MIN_RECIPES:
        print(f"{summary} — fewer than {MIN_RECIPES}, nothing written", file=sys.stderr)
        return 1

    pack = {
        "source": SOURCE,
        "home": HOME,
        "seen": time.strftime("%Y-%m-%d"),
        "recipes": recipes,
    }
    with open(argv[1], "w", encoding="utf-8", newline="\n") as out:
        json.dump(pack, out, ensure_ascii=False, indent=1)
        out.write("\n")
    print(summary)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
