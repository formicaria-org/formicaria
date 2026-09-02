#!/bin/sh
# Pull down a GitHub Actions run's **logs and artifacts** so they can be read locally.
#
# Written for `ios.yml` (rung 1), which is the workflow this project cannot debug any other way:
# there is no Mac and no iPhone, so a run's log is the entire observation. It takes any workflow
# though — `sh ci/ios-logs.sh cross.yml` works.
#
# **Two backends, because neither is guaranteed.** `gh` is used when it is on PATH; otherwise this
# falls back to the REST API with `curl`, which needs a token in `GH_TOKEN` or `GITHUB_TOKEN` with
# `actions:read`. The repo is private, so an unauthenticated fetch cannot work — the script says so
# rather than handing back a 404 to interpret.
#
# Output lands in `.ci-logs/<workflow>-<run-id>/`, which is gitignored. Logs and artifacts are
# **both** fetched: for rung 1 the artifact is `gen/apple/`, and whether it exists at all is one of
# the three questions the run is being asked.
#
#   sh ci/ios-logs.sh              # newest ios.yml run
#   sh ci/ios-logs.sh ios.yml 42   # a specific run id
set -eu

WORKFLOW="${1:-ios.yml}"
RUN_ID="${2:-}"
REPO="${FM_GH_REPO:-formicaria-org/formicaria}"
OUT_ROOT="${FM_CI_LOGS:-.ci-logs}"

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"

have() { command -v "$1" >/dev/null 2>&1; }

# ---- gh, when it is there. It handles auth, redirects and unzipping on its own. --------------
if have gh; then
    if ! gh auth status >/dev/null 2>&1; then
        echo "ios-logs: gh is installed but not logged in — run 'gh auth login' first" >&2
        exit 1
    fi
    if [ -z "$RUN_ID" ]; then
        RUN_ID=$(gh run list --workflow="$WORKFLOW" -L 1 --json databaseId \
            --jq '.[0].databaseId' -R "$REPO" 2>/dev/null || true)
    fi
    [ -n "$RUN_ID" ] || { echo "ios-logs: no runs found for $WORKFLOW — has it been fired yet?" >&2; exit 1; }

    dest="$OUT_ROOT/${WORKFLOW%.yml}-$RUN_ID"
    mkdir -p "$dest"

    # Conclusion first: it decides how to read everything below it.
    gh run view "$RUN_ID" -R "$REPO" > "$dest/summary.txt" 2>&1 || true
    # `|| true` on the log fetch: a *failed* run is the interesting case and `gh run view --log`
    # exits non-zero on one. Losing the log because the run went red would be exactly backwards.
    gh run view "$RUN_ID" -R "$REPO" --log > "$dest/full.log" 2>&1 || true
    gh run view "$RUN_ID" -R "$REPO" --log-failed > "$dest/failed.log" 2>&1 || true
    gh run download "$RUN_ID" -R "$REPO" -D "$dest/artifacts" 2>/dev/null \
        || echo "ios-logs: no artifacts on this run (for rung 1 that is itself an answer)"
else
    # ---- curl fallback. -----------------------------------------------------------------------
    have curl || { echo "ios-logs: needs either 'gh' or 'curl' on PATH" >&2; exit 1; }
    have unzip || { echo "ios-logs: needs 'unzip' on PATH for the curl path" >&2; exit 1; }
    TOKEN="${GH_TOKEN:-${GITHUB_TOKEN:-}}"
    if [ -z "$TOKEN" ]; then
        echo "ios-logs: no 'gh' on PATH and no GH_TOKEN/GITHUB_TOKEN set." >&2
        echo "          This repo is private, so an anonymous fetch returns 404, not logs." >&2
        echo "          Either 'pixi global install gh && gh auth login', or export a token" >&2
        echo "          with actions:read." >&2
        exit 1
    fi
    api() { curl -sSL -H "Authorization: Bearer $TOKEN" \
                 -H "Accept: application/vnd.github+json" "$@"; }

    if [ -z "$RUN_ID" ]; then
        RUN_ID=$(api "https://api.github.com/repos/$REPO/actions/workflows/$WORKFLOW/runs?per_page=1" \
            | tr ',' '\n' | grep -m1 '"id"' | tr -dc '0-9')
    fi
    [ -n "$RUN_ID" ] || { echo "ios-logs: no runs found for $WORKFLOW (or the token cannot see them)" >&2; exit 1; }

    dest="$OUT_ROOT/${WORKFLOW%.yml}-$RUN_ID"
    mkdir -p "$dest"

    api "https://api.github.com/repos/$REPO/actions/runs/$RUN_ID" > "$dest/summary.json"
    api "https://api.github.com/repos/$REPO/actions/runs/$RUN_ID/logs" -o "$dest/logs.zip"
    unzip -oq "$dest/logs.zip" -d "$dest/logs" && rm -f "$dest/logs.zip"

    # Artifacts are a separate endpoint and a separate zip each.
    api "https://api.github.com/repos/$REPO/actions/runs/$RUN_ID/artifacts" > "$dest/artifacts.json"
    for id in $(tr ',' '\n' < "$dest/artifacts.json" | grep -A1 '"artifacts"' | grep '"id"' | tr -dc '0-9\n'); do
        [ -n "$id" ] || continue
        api "https://api.github.com/repos/$REPO/actions/artifacts/$id/zip" -o "$dest/artifact-$id.zip" || continue
        unzip -oq "$dest/artifact-$id.zip" -d "$dest/artifacts" && rm -f "$dest/artifact-$id.zip"
    done
fi

echo "ios-logs: $WORKFLOW run $RUN_ID -> $dest"
find "$dest" -maxdepth 2 -type f | head -20
