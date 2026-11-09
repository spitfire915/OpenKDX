#!/usr/bin/env bash
# Common scaffolding for capture scenarios. Each scenario script sources this,
# then calls:   scenario_begin NAME  …steps…  scenario_end
# Steps are recorded into $CAPTURES/NAME.md as they run (via `step "…"`).
set -euo pipefail

HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
LAB="$HERE/kdx-lab"
# shellcheck disable=SC1091
source "$HERE/lib/ui.sh"
CAPTURES="${OPENKDX_LAB_CAPTURES:-$HOME/captures}"
RUN="${OPENKDX_LAB_RUN:-$HOME/lab/run}"
# shellcheck disable=SC1090
source "${OPENKDX_LAB_ENV:-$HOME/haxial/lab.env}"

SCENARIO=""
STEP_N=0
NOTES=""

step() {
  echo "  - $*"
  NOTES+="$(printf '%s. %s\n' "$((++STEP_N))" "$*")"$'\n'
}

scenario_begin() {
  SCENARIO=$1; STEP_N=0
  echo "scenario $SCENARIO:"
  "$LAB" ns stop >/dev/null 2>&1 || true
  "$LAB" reset all >/dev/null 2>&1 || true
  "$LAB" ns start >/dev/null
  "$LAB" x start >/dev/null
}

# Fresh server, wait until it listens.
start_server() {
  "$LAB" server start >/dev/null
  for _ in $(seq 1 50); do grep -q "KDX Server started" "$RUN/server.log" && break; sleep 0.2; done
  step "Start KDX Server 1.620 (Linux) in the sandbox; first-run setup creates admin '$KDX_ADMIN_LOGIN', guest access enabled."
}

start_tracker() {
  "$LAB" tracker start >/dev/null
  sleep 2
  step "Start KDX Tracker 1.110 (Linux) in the sandbox."
}

# Fresh Windows client through licence + name dialogs; leaves the palette up.
WINCLASS=kdxclient.exe
LNXCLASS=KDX
start_winclient() {
  local name=${1:-labuser}
  "$LAB" winclient start >/dev/null
  ui_first_run_client "$name" $WINCLASS
  PALETTE=$(ui_wait_sized KDX 164x310 20 $WINCLASS) || { echo "no palette window" >&2; exit 1; }
  CLASS=$WINCLASS
  step "Start KDX Client 1.600 (Windows, under Wine); accept the licence; set user name '$name'."
}

# Fresh Linux client (1.620 beta 5), same dialogs.
start_lnxclient() {
  local name=${1:-lnxuser}
  "$LAB" client start >/dev/null
  ui_first_run_client "$name" $LNXCLASS
  PALETTE=$(ui_wait_sized KDX 164x310 20 $LNXCLASS) || { echo "no palette window" >&2; exit 1; }
  CLASS=$LNXCLASS
  # Keep the two clients apart on the shared display: Linux lives on the right.
  for id in $(X xdotool search --class "^$LNXCLASS\$" 2>/dev/null || true); do
    read -r x y <<<"$(X xdotool getwindowgeometry --shell "$id" | awk -F= '/^X=/{x=$2}/^Y=/{y=$2}END{print x, y}')"
    [[ "$x" -lt 640 ]] && X xdotool windowmove "$id" $((x + 640)) "$y" 2>/dev/null || true
  done
  PALETTE=$(ui_win_sized KDX 164x310 $LNXCLASS)
  step "Start KDX Client 1.620 beta 5 (Linux); accept the licence; set user name '$name'."
}

# use_client win|lnx : make subsequent palette_click/connect_to target that client.
use_client() {
  case $1 in
    win) CLASS=$WINCLASS;; lnx) CLASS=$LNXCLASS;;
  esac
  PALETTE=$(ui_win_sized KDX 164x310 $CLASS)
}

# Palette buttons (164x310 window), y centres.
palette_click() {
  local which=$1 y
  case $which in
    commands) y=79;; connect) y=103;; addressbook) y=124;; messages) y=145;;
    transfers) y=166;; files) y=187;; chat) y=208;; news) y=229;; userlist) y=250;; exit) y=271;;
    *) echo "palette_click: unknown $which" >&2; return 1;;
  esac
  ui_focus "$PALETTE"; ui_click "$PALETTE" 80 "$y"; sleep 1
}

# Connect dialog: TYPE is left at Server.
connect_to() { # connect_to ADDRESS LOGIN PASSWORD
  local c
  for _ in 1 2 3; do
    [[ -n "$(ui_win "KDX Connect" "${CLASS:-}")" ]] && break
    palette_click connect
    ui_wait "KDX Connect" 8 "${CLASS:-}" && break
  done
  c=$(ui_win "KDX Connect" "${CLASS:-}"); [[ -n "$c" ]] || { echo "connect dialog did not open" >&2; return 1; }
  ui_focus "$c"
  ui_click "$c" 240 97;  ui_type "$1"
  ui_click "$c" 240 127; [[ -n "$2" ]] && ui_type "$2"
  ui_click "$c" 240 157; [[ -n "$3" ]] && ui_type "$3"
  ui_click "$c" 312 190
  sleep 5
  # A second dialog from a retried click would otherwise linger.
  for c in $(X xdotool search --name '^KDX Connect$' 2>/dev/null || true); do ui_close "$c"; done
  local pw="<empty>"; [[ -n "$3" ]] && pw="(set)"
  step "Connect → address '$1', login '${2:-<empty>}', password $pw; press Connect."
}

capture_begin() { "$LAB" capture start "$SCENARIO" >/dev/null; step "Start capturing loopback."; }
capture_end() { "$LAB" capture stop | tail -1; }

scenario_end() {
  local frames
  frames=$(tshark -r "$CAPTURES/$SCENARIO.pcapng" 2>/dev/null | wc -l | tr -d ' ')
  ui_shot "$SCENARIO" || true
  {
    echo "# Capture: $SCENARIO"
    echo
    echo "Recorded $(date -u +%Y-%m-%d) in the lab (\`lab/scenarios/$SCENARIO.sh\`)."
    echo "Programs: ${PROGRAMS:-KDX Server 1.620 (Linux), KDX Client 1.600 (Windows under Wine 6.0.3)}."
    echo "Frames: $frames. Screenshot at the end: \`$SCENARIO.png\`."
    echo
    echo "## Steps"
    echo
    printf '%s' "$NOTES"
    echo
    echo "## Server log"
    echo
    echo '```'
    sed 's/\x1b\[[0-9;]*[A-Za-z]//g' "$RUN/server.log" | tr -d '\r' | head -40
    echo '```'
  } > "$CAPTURES/$SCENARIO.md"
  "$LAB" ns stop >/dev/null
  echo "scenario $SCENARIO: done ($frames frames)"
}
