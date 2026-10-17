#!/bin/sh
# **The collaboration lab: one shared vault, three devices, two people — all on this machine.**
#
#   pixi run collab-lab start      set up (first time) and start the three devices
#   pixi run collab-lab status     who is running, and how far each device is from the shared copy
#   pixi run collab-lab checklist  the scenarios worth trying, in order
#   pixi run collab-lab stop       stop the three programs (the vaults stay)
#   pixi run collab-lab reset      stop, and delete the whole lab
#
# | device     | who | address                |
# |------------|-----|------------------------|
# | ana-laptop | Ana | http://127.0.0.1:8801  |
# | ana-phone  | Ana | http://127.0.0.1:8802  |  same person, a second device
# | ben-laptop | Ben | http://127.0.0.1:8803  |  a second person
#
# **Why it exists** (2026-10-17). The owner's phone held 46 changes for five days without sending
# them: `isRemote()` took the phone for a paired tablet, so backing up by itself never started.
# The Rust suites already run two devices against a real remote at the *command* level
# (`fm-cli/tests/mixed_device_collaboration.rs`), so what had never been exercised was the running
# app — its timers, its checks, its buttons. Here each device is a real `fm-serve`, driven by a
# person in a browser window, so every automatic behaviour runs exactly as it does at home.
#
# **What it can never touch.** The shared copy is a bare repo inside the lab directory, so nothing
# goes over a network and no real remote is involved. Each device has its own config directory
# (`XDG_CONFIG_HOME` — vaults.json, tokens, the restic password, reminders), its own git config
# (`GIT_CONFIG_GLOBAL`, so a signing rule in yours cannot reach in) and its own port, which is its
# own browser origin and so its own `localStorage` (the "back up by itself" switch is per device).
# Names and addresses are invented; `.invalid` is reserved and cannot be anyone's.
#
# **Its limit, said plainly.** `ana-phone` is a browser tab, not the Android shell, and a Linux
# `fm-serve` has no libgit2 — so the phone's git engine and anything specific to the Tauri shell
# (today's bug among them) are not here. Those are `App.autoBackup.test.ts`, `asPhone()` and the
# Rust differential suites.
set -eu

LAB=${FM_LAB_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/formicaria-lab}
BIN=${FM_LAB_BIN:-$(pwd)/target/release}
MARK="$LAB/.formicaria-lab"

# device|port|name|email
DEVICES='ana-laptop|8801|Ana|ana@lab.invalid
ana-phone|8802|Ana|ana@lab.invalid
ben-laptop|8803|Ben|ben@lab.invalid'

each() {
  printf '%s\n' "$DEVICES" | while IFS='|' read -r dev port name email; do
    "$@" "$dev" "$port" "$name" "$email"
  done
}

running() { # pidfile -> exit 0 if that pid is an fm-serve that is alive
  [ -f "$1" ] || return 1
  pid=$(cat "$1")
  [ "$(ps -p "$pid" -o comm= 2>/dev/null)" = "fm-serve" ]
}

isolated_git() { GIT_CONFIG_GLOBAL="$LAB/gitconfig" GIT_CONFIG_NOSYSTEM=1 git "$@"; }

write_config() { # dev
  mkdir -p "$LAB/$1/config/formicaria"
  printf '{\n  "vaults": [\n    { "name": "lab", "path": "%s" }\n  ]\n}\n' "$LAB/$1/vault" \
    >"$LAB/$1/config/formicaria/vaults.json"
}

api() { # port cmd json -> the server's answer
  curl -sf -X POST "http://127.0.0.1:$1/api/$2" -H 'content-type: application/json' \
    -H 'x-formicaria-tab: collab-lab' -H "origin: http://127.0.0.1:$1" -d "$3"
}

wait_up() { # port
  for _ in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20; do
    api "$1" backup_status '{}' >/dev/null 2>&1 && return 0
    sleep 0.5
  done
  echo "collab-lab: the device on :$1 did not answer; see its serve.log" >&2
  exit 1
}

join_device() { # dev port name email — a later device: a copy of what the first one sent
  d="$LAB/$1"
  [ -d "$d/vault" ] && return 0
  isolated_git clone -q "$LAB/shared.git" "$d/vault"
  isolated_git -C "$d/vault" config user.name "$3"
  isolated_git -C "$d/vault" config user.email "$4"
  write_config "$1"
}

# **The first device sets the vault up through the app, as it would at home.** Seeding by hand
# (only `notes/` committed) left every device to write its own untracked `.gitignore` and
# `.gitattributes`, and the first get on a second device then refused: "untracked working tree
# files would be overwritten by merge". Real vaults never start like that — the first device's
# first backup carries those files — so the lab now starts the way they do.
setup() {
  [ -f "$MARK" ] && return 0
  if [ -e "$LAB" ] && [ -n "$(ls -A "$LAB" 2>/dev/null)" ]; then
    echo "collab-lab: $LAB exists and is not a lab — refusing to use it." >&2
    exit 1
  fi
  mkdir -p "$LAB"
  : >"$LAB/gitconfig"
  isolated_git init -q --bare -b main "$LAB/shared.git"
  v="$LAB/ana-laptop/vault"
  "$BIN/fm" --vault "$v" capture "Welcome to the lab. Open this note on two devices and edit it on both." >/dev/null
  "$BIN/fm" --vault "$v" capture "Reading list" >/dev/null
  "$BIN/fm" --vault "$v" capture "Shared tasks" >/dev/null
  isolated_git -C "$v" init -q -b main
  isolated_git -C "$v" config user.name Ana
  isolated_git -C "$v" config user.email ana@lab.invalid
  isolated_git -C "$v" remote add origin "$LAB/shared.git"
  write_config ana-laptop
  start_device ana-laptop 8801
  wait_up 8801
  api 8801 commit '{"message":"backup: the lab starts","vault":"lab"}' >/dev/null
  api 8801 push '{"message":"backup: the lab starts","vault":"lab"}' >/dev/null
  each join_device
  : >"$MARK"
}

start_device() { # dev port name email
  d="$LAB/$1"
  if running "$d/pid"; then
    :
    return 0
  fi
  env XDG_CONFIG_HOME="$d/config" FM_VAULTS="$d/config/formicaria/vaults.json" \
    FM_ADDR="127.0.0.1:$2" FM_OPEN=0 FM_AUTO_SHUTDOWN=0 \
    GIT_CONFIG_GLOBAL="$LAB/gitconfig" GIT_CONFIG_NOSYSTEM=1 \
    nohup "$BIN/fm-serve" >"$d/serve.log" 2>&1 &
  echo $! >"$d/pid"
}

stop_device() { # dev port name email
  p="$LAB/$1/pid"
  if running "$p"; then
    kill "$(cat "$p")"
    echo "  stopped $1"
  fi
  rm -f "$p"
}

status_device() { # dev port name email
  d="$LAB/$1"
  state=stopped
  running "$d/pid" && state="running on http://127.0.0.1:$2"
  isolated_git -C "$d/vault" fetch -q origin 2>/dev/null || true
  counts=$(isolated_git -C "$d/vault" rev-list --left-right --count HEAD...origin/main 2>/dev/null || echo "? ?")
  ahead=${counts%%	*}
  behind=${counts##*	}
  dirty=$(isolated_git -C "$d/vault" status --porcelain -- notes 2>/dev/null | wc -l | tr -d ' ')
  printf '  %-11s %-4s %-32s not sent: %-3s not got: %-3s unsaved files: %s\n' \
    "$1" "$3" "$state" "$ahead" "$behind" "$dirty"
}

checklist() {
  cat <<'EOF'
Ana on two devices (ana-laptop :8801, ana-phone :8802) — nothing should need a press:
  1. New note on one device. Within ~2 min it is on the other, and search finds it there.
  2. Edit one note on both devices, a minute apart. Both edits survive; nothing is lost.
  3. Edit the same sentence on both before either has sent. What does the conflict say,
     and is it worded as "someone else" when it is Ana on her other device?
  4. Open a note and leave it without changing anything. Did that device send a change?
  5. Delete a note on one device while it is open and being edited on the other.
  6. Set the same field (e.g. a date) to different values on the two devices.
  7. Stop one device mid-edit (collab-lab stop), edit on the other, start again: does it catch up?

Ana and Ben (add ben-laptop :8803):
  8. Repeat 1–3 between ana-laptop and ben-laptop: the wording should now say it is Ben.
  9. Proposals: create one on Ben's side, review and accept or reject on Ana's.
 10. Ben changes a note Ana has open: what does Ana's open note do?

Between steps: pixi run collab-lab status   (not sent / not got per device)
EOF
}

case "${1:-}" in
start)
  for b in fm fm-serve; do
    [ -x "$BIN/$b" ] || { echo "collab-lab: no $BIN/$b — run: pixi run build" >&2; exit 1; }
  done
  # The UI is baked into the binary, so a stale binary is a stale app.
  newer=$(find ui/src crates -newer "$BIN/fm-serve" -type f \( -name '*.ts' -o -name '*.svelte' -o -name '*.rs' \) ! -name '*.test.ts' ! -path '*/tests/*' 2>/dev/null | head -1)
  [ -n "$newer" ] && echo "collab-lab: note — $newer is newer than the binary; pixi run build for current code."
  setup
  each start_device
  sleep 2
  echo "collab-lab: $LAB"
  each status_device
  echo "Open each address in its own browser window. Scenarios: pixi run collab-lab checklist"
  ;;
stop) [ -f "$MARK" ] && each stop_device ;;
status)
  [ -f "$MARK" ] || { echo "collab-lab: no lab yet — pixi run collab-lab start"; exit 0; }
  each status_device
  ;;
checklist) checklist ;;
reset)
  if [ -f "$MARK" ]; then
    each stop_device
    rm -rf "$LAB"
    echo "collab-lab: removed $LAB"
  fi
  ;;
*)
  sed -n '2,9p' "$0" | sed 's/^# \{0,1\}//'
  exit 1
  ;;
esac
