#!/bin/sh
# Regenerate the manual's screenshots, from a throwaway demo vault.
#
# **The manual is for a GUI and had no pictures in it at all.** `outstanding.md` §2.7 called that
# the largest gap by distance, and it is: every chapter describes a screen the reader cannot see.
#
# **Everything here is disposable, and that is the point.** The vault is built in a temp directory
# from the notes below, served on a spare port, captured and thrown away. Nothing touches the
# user's real vault, the real config, or port 8765 — a screenshot script that could photograph
# somebody's actual notes is one nobody should run.
#
#   pixi run shots
#
# The notes are deliberately dull-but-plausible research chores. They exist to fill a board with
# three columns, put items on an agenda in a readable spread, and give the read view something
# worth showing (a Mermaid diagram, which renders offline and is a headline claim).
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"

command -v chromium >/dev/null 2>&1 || {
    echo "shots: chromium is not on PATH — it is the capture browser (see ci/shots.py)" >&2
    exit 1
}

work=$(mktemp -d)
port=${FM_SHOT_PORT:-8791}
cleanup() {
    [ -n "${serve_pid:-}" ] && kill "$serve_pid" 2>/dev/null || true
    rm -rf "$work"
}
trap cleanup EXIT INT TERM

[ -x target/release/fm-serve ] || {
    echo "shots: building first (pixi run build)…"
    pixi run build >/dev/null
}

vault="$work/vault"
mkdir -p "$vault/notes"

note() { # id title status due tags created body
    {
        echo "---"
        echo "id: $1"
        echo "type: note"
        echo "title: $2"
        echo "status: $3"
        [ -n "$4" ] && echo "due: $4"
        echo "tags: [$5]"
        echo "created: $6"
        echo "updated: $6"
        echo "---"
        echo
        printf '%s\n' "$7"
    } > "$vault/notes/$1.md"
}

note 01KZ000000000000000000000A "Reproduce the CRLF parse failure" doing 2026-09-08 "lab, bug" 2026-08-28T09:12:00Z \
'On a Windows checkout every note failed to parse and they *vanished* rather than erroring,
because the loader is deliberately tolerant.

Next: add a fixture with CRLF endings and check `from_file` keeps both.'
note 01KZ000000000000000000000B "Draft the methods section" doing 2026-09-11 "paper, shelf:Thesis" 2026-08-29T14:03:00Z \
'Three paragraphs on the sampling design. The inclusion criteria still need a number.

Inline maths renders with no network access: $E = mc^2$.'
note 01KZ000000000000000000000C "Read Chen et al. on sparse retrieval" todo 2026-09-06 "reading, paper, shelf:Reading" 2026-08-30T08:40:00Z \
'Claimed 3x recall at equal latency. Check whether the baseline was tuned.'
note 01KZ000000000000000000000D "Supervisor meeting" todo 2026-09-05 "lab" 2026-09-01T11:00:00Z \
'Bring: the pilot numbers, the revised timeline, and the question about the second cohort.'
note 01KZ000000000000000000000E "Pilot analysis" done "" "lab, paper, shelf:Thesis" 2026-08-25T16:20:00Z \
'Ran the mixed model. Effect holds after excluding the two incomplete sessions.'
note 01KZ000000000000000000000F "Set up the shared lab vault" done "" "lab" 2026-08-22T10:05:00Z \
'One repository, three of us. Access is who can clone it — there is no per-note permission and
there does not need to be.'
note 01KZ000000000000000000000G "Reading list for the review" todo 2026-09-19 "reading, shelf:Reading" 2026-09-02T09:00:00Z \
'Twelve papers, four of them behind the same paywall.

- [ ] Chen et al.
- [ ] Okonkwo & Baptiste
- [ ] the 2024 survey'
note 01KZ000000000000000000000H "Notation for the appendix" todo "" "paper, shelf:Thesis" 2026-09-03T15:30:00Z \
'Diagrams render from a fenced block, offline:

```mermaid
flowchart LR
  Notes --> Index --> Search
  Notes --> Git --> Collaborator
```'

# A git identity, or the app opens on the first-run welcome screen instead of the workspace —
# which is a fine screenshot and not the one most of these chapters need.
git -C "$vault" init -q
git -C "$vault" config user.name "Ada Lovelace"
git -C "$vault" config user.email "ada@example.org"
printf 'index.sqlite\nderived/\nblobs/\n.fm-ingest/\n' > "$vault/.gitignore"
git -C "$vault" add -A >/dev/null
git -C "$vault" -c commit.gpgsign=false commit -qm "the demo vault"

printf '{ "vaults": [ { "name": "research", "path": "%s" } ] }\n' "$vault" > "$work/vaults.json"

# **A home of its own, too** (2026-10-24): the vault was already throwaway, but the per-machine
# settings — the assistant's state, its device name, the model on disk, reminders — are read from
# the home folder, and a shot of Settings → Assistant photographed this machine's. A new user's is
# what the manual shows. Started from `$work`, not the checkout: a dev run takes `./agents` (the
# checkout's models) as its own, and the shot would show what is on this disk.
mkdir -p "$work/home/.config/formicaria/tools"
# The catalogue a release carries beside its binary, so the assistant reads as a fresh install's —
# off, nothing downloaded — rather than "cannot find its list of models".
cp agents/models.toml "$work/home/.config/formicaria/tools/models.toml"
(cd "$work" && exec env HOME="$work/home" XDG_CONFIG_HOME="$work/home/.config" \
    XDG_DATA_HOME="$work/home/.local/share" XDG_CACHE_HOME="$work/home/.cache" \
    FM_VAULTS="$work/vaults.json" FM_ADDR="127.0.0.1:$port" FM_OPEN=0 FM_AUTO_SHUTDOWN=0 \
    "$root/target/release/fm-serve") > "$work/serve.log" 2>&1 &
serve_pid=$!

i=0
until curl -fs -o /dev/null "http://127.0.0.1:$port/"; do
    i=$((i + 1))
    [ "$i" -lt 60 ] || { echo "shots: fm-serve did not come up; see $work/serve.log" >&2; exit 1; }
    sleep 0.5
done

FM_SHOT_URL="http://127.0.0.1:$port/" python3 ci/shots.py
echo "shots: wrote docs/src/images/"
