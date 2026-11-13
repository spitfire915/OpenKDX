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
# Creates lab.env with the published lab password if it does not exist yet.
"$LAB" env >/dev/null
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
  # One lab session at a time: refuse if another holds the lock or a namespace is up.
  exec 9>"$RUN/lab.lock"
  flock -n 9 || { echo "scenario: the lab is in use (lock $RUN/lab.lock)" >&2; exit 1; }
  if "$LAB" ns status | grep -q running && [[ "${FORCE:-0}" != 1 ]]; then
    echo "scenario: a lab session is running; stop it (kdx-lab ns stop) or set FORCE=1" >&2; exit 1
  fi
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

# --- The OpenKDX command-line client (crates/openkdx-cli) ---------------------
# use_cli builds it once and makes cli/cli_guest/cli_run drive it inside the
# namespace. Its transcript goes to $RUN/cli.out for the scenario notes.
CLI_BIN=""
use_cli() {
  export PATH="$HOME/.cargo/bin:$PATH"
  cargo build -q -p openkdx-cli --manifest-path "$HERE/../Cargo.toml"
  CLI_BIN="$HERE/../target/debug/openkdx"
  [[ -x "$CLI_BIN" ]] || { echo "openkdx CLI did not build" >&2; exit 1; }
  if [[ -z "${OPENKDX_KDX_TABLE:-}" ]]; then
    OPENKDX_KDX_TABLE="$(base64 -w0 "$(cargo run -q -p openkdx-table --manifest-path "$HERE/../Cargo.toml" -- path 2>/dev/null | head -1)/kdx.bin")"
    export OPENKDX_KDX_TABLE
  fi
  PROGRAMS="${PROGRAMS:-KDX Server 1.620 (Linux), OpenKDX client (openkdx CLI, $(git -C "$HERE/.." rev-parse --short HEAD))}"
  : > "$RUN/cli.out"
}

# cli [--as LOGIN PASSWORD] [--nick NICK] ARGS... : one command, admin by default.
cli() {
  local login="$KDX_ADMIN_LOGIN" pw="$KDX_ADMIN_PASSWORD" nick="openkdx"
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --as) login=$2; pw=$3; shift 3 ;;
      --nick) nick=$2; shift 2 ;;
      *) break ;;
    esac
  done
  "$LAB" exec env OPENKDX_KDX_TABLE="$OPENKDX_KDX_TABLE" "$CLI_BIN"     -s 127.0.0.1 -l "$login" -p "$pw" --nick "$nick" "$@" 2>&1 | tee -a "$RUN/cli.out"
  return "${PIPESTATUS[0]}"
}

cli_guest() { cli --as "" "" "$@"; }

# cli_run [cli options] <<'EOF' … EOF : many commands on one session (openkdx run).
cli_run() { cli "$@" run --keep-going; }

# cli_bg NAME [cli options] ARGS... : in the background; wait with cli_wait NAME.
cli_bg() {
  local name=$1; shift
  _cli "$@" > "$RUN/cli-$name.out" 2>&1 &
  echo $! > "$RUN/cli-$name.pid"
  sleep 1
}
cli_wait() {
  local name=$1 rc=0
  wait "$(cat "$RUN/cli-$name.pid")" || rc=$?
  cat "$RUN/cli-$name.out" >> "$RUN/cli.out"
  return $rc
}
# For the notes: the background client's transcript.
cli_bg_out() { cat "$RUN/cli-$1.out"; }
capture_end() { "$LAB" capture stop | tail -1; }

scenario_end() {
  local frames
  frames=$(tshark -r "$CAPTURES/$SCENARIO.pcapng" 2>/dev/null | wc -l | tr -d ' ')
  ui_shot "$SCENARIO" >/dev/null 2>&1 || true   # stays on the lab machine; never committed
  {
    echo "# Capture: $SCENARIO"
    echo
    echo "Recorded $(date -u +%Y-%m-%d) in the lab (\`lab/scenarios/$SCENARIO.sh\`)."
    echo "Programs: ${PROGRAMS:-KDX Server 1.620 (Linux), KDX Client 1.600 (Windows under Wine 6.0.3)}."
    echo "Frames: $frames."
    echo
    echo "## Steps"
    echo
    printf '%s' "$NOTES"
    echo
    echo "## Server log"
    echo
    echo '```'
    sed 's/\x1b\[[0-9;]*[A-Za-z]//g' "$RUN/server.log" | tr -d '\r' | head -60
    echo '```'
    if [[ -n "$CLI_BIN" && -s "$RUN/cli.out" ]]; then
      echo
      echo "## Client transcript"
      echo
      echo '```'
      head -80 "$RUN/cli.out"
      echo '```'
    fi
  } > "$CAPTURES/$SCENARIO.md"
  # The decoded message inventory; crates/openkdx-dissect/tests/captures.rs checks it.
  if command -v cargo >/dev/null 2>&1 || [[ -x "$HOME/.cargo/bin/cargo" ]]; then
    {
      echo
      echo "## Messages"
      echo
      echo "Decoded by \`openkdx-dissect --inventory -p openkdx-lab captures/$SCENARIO.pcapng\`; checked by \`crates/openkdx-dissect/tests/captures.rs\`."
      echo
      PATH="$HOME/.cargo/bin:$PATH" cargo run -q -p openkdx-dissect --manifest-path "$HERE/../Cargo.toml" -- \
        --inventory -p "$KDX_ADMIN_PASSWORD" "$CAPTURES/$SCENARIO.pcapng" 2>/dev/null \
        || echo "(openkdx-dissect failed; run it by hand and paste the inventory here)"
    } >> "$CAPTURES/$SCENARIO.md"
  fi
  "$LAB" ns stop >/dev/null
  echo "scenario $SCENARIO: done ($frames frames)"
}
