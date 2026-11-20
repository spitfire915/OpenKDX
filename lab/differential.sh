#!/usr/bin/env bash
# Differential tests against the original KDX Server 1.620 and Tracker 1.110
# in the sandbox: the kdx-client suite (crates/kdx-client/tests/lab.rs of the
# kdx checkout) and, until Phase 3E deletes it, the openkdx-client suite
# (crates/openkdx-client/tests/lab.rs). Runs on the lab machine, as the
# openkdx user. Exit non-zero on any failure.
#
#   lab/differential.sh [KDX_SRC]    KDX_SRC defaults to ~/kdx-src; lab/kdx-rev.sh
#                                    puts it at the pinned revision
#   lab/differential.sh --supports-kdx-src   exits 0: the fork's lab job probes this
#   lab/differential.sh --lock-only  takes the lab lock, holds it 5 s, exits (lab/lock-test.sh)
#
# The lock: two runners (OpenKDX's and the fork's) may start within seconds
# of each other. With LAB_WAIT=1, which both workflows set, the loser waits
# up to 40 minutes; without it, the loser fails at once.
set -euo pipefail

[[ "${1:-}" == --supports-kdx-src ]] && exit 0

HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/.." && pwd)
LAB="$HERE/kdx-lab"
RUN="${OPENKDX_LAB_RUN:-$HOME/lab/run}"
export PATH="$HOME/.cargo/bin:$PATH"

take_lock() {
  exec 9>"$RUN/lab.lock"
  if [[ "${LAB_WAIT:-0}" == 1 ]]; then
    flock -w 2400 9 || { echo "differential: the lab stayed busy for 40 min (lock $RUN/lab.lock)" >&2; exit 1; }
  else
    flock -n 9 || { echo "differential: the lab is in use (lock $RUN/lab.lock); not running" >&2; exit 1; }
  fi
}

if [[ "${1:-}" == --lock-only ]]; then
  take_lock
  echo "lock-only: holding $RUN/lab.lock for 5 s"
  sleep 5
  exit 0
fi

KDX_SRC="${1:-${OPENKDX_KDX_SRC:-$HOME/kdx-src}}"
[[ -f "$KDX_SRC/Cargo.toml" ]] || { echo "differential: no kdx checkout at $KDX_SRC (lab/kdx-rev.sh sync)" >&2; exit 1; }

cleanup() { "$LAB" ns stop >/dev/null 2>&1 || true; }

take_lock
# Holding the lock, a namespace that is still up was left by a run that died.
"$LAB" ns status | grep -q running && echo "differential: a namespace was left running; stopping it" >&2
trap cleanup EXIT

# The challenge table, from the cache or the server binary (as ci.yml does).
cargo run -q -p openkdx-table --manifest-path "$REPO/Cargo.toml" -- path | grep -q "kdx.bin: present" \
  || cargo run -q -p openkdx-table --manifest-path "$REPO/Cargo.toml" -- import \
       "$HOME/haxial/bin/KDXServer1620-Lnx/KDXServer1620-Lnx/KDXServer.lexe"
TABLE_DIR=$(cargo run -q -p openkdx-table --manifest-path "$REPO/Cargo.toml" -- path 2>/dev/null | sed -n 1p)
OPENKDX_KDX_TABLE="$(base64 -w0 "$TABLE_DIR/kdx.bin")"
KDX_CHALLENGE_TABLE="$OPENKDX_KDX_TABLE"
export OPENKDX_KDX_TABLE KDX_CHALLENGE_TABLE

# The test binary of one crate's `lab` test target.
test_binary() {
  local manifest=$1 crate=$2
  cargo test -q --manifest-path "$manifest" -p "$crate" --test lab --no-run --message-format=json 2>/dev/null \
    | python3 -c 'import json,sys
for line in sys.stdin:
    try: m=json.loads(line)
    except ValueError: continue
    if m.get("reason")=="compiler-artifact" and m.get("executable") and m["target"]["name"]=="lab":
        print(m["executable"])'
}

# Build outside the namespace (no network inside it).
echo "differential: kdx checkout $KDX_SRC at $(git -C "$KDX_SRC" rev-parse --short HEAD)"
KDX_BIN=""
if [[ -f "$KDX_SRC/crates/kdx-client/tests/lab.rs" ]]; then
  KDX_BIN=$(test_binary "$KDX_SRC/Cargo.toml" kdx-client)
  [[ -x "$KDX_BIN" ]] || { echo "differential: kdx-client lab test binary not found" >&2; exit 1; }
else
  # A revision from before the suite landed (the pin lags the fork by one bump).
  echo "differential: no crates/kdx-client/tests/lab.rs in $KDX_SRC; kdx-client suite skipped"
fi
OPENKDX_BIN=""
if [[ -d "$REPO/crates/openkdx-client" ]]; then
  OPENKDX_BIN=$(test_binary "$REPO/Cargo.toml" openkdx-client)
  [[ -x "$OPENKDX_BIN" ]] || { echo "differential: openkdx-client lab test binary not found" >&2; exit 1; }
fi

# Fresh server and tracker; the server registers with the tracker on startup.
# Each suite gets its own server start: the server bans an address after 30
# connections in 300 s, and a suite opens up to 25.
start_lab() {
  "$LAB" ns stop >/dev/null 2>&1 || true
  "$LAB" reset server >/dev/null 2>&1 || true
  "$LAB" reset tracker >/dev/null 2>&1 || true
  "$LAB" ns start
  "$LAB" tracker start
  sleep 1
  "$LAB" server start
  for _ in $(seq 1 50); do
    grep -q "KDX Server started" "$RUN/server.log" && break
    sleep 0.2
  done
  grep -q "KDX Server started" "$RUN/server.log" || { "$LAB" server log; echo "differential: server did not start" >&2; exit 1; }
  sleep 2
}

# The client presents itself as a registered Client 1.600 (design §9): the
# server must not log a licence reminder.
check_log() {
  echo "--- server log (tail)"
  sed 's/\x1b\[[0-9;]*[A-Za-z]//g' "$RUN/server.log" | tail -40
  if grep -qi "licen" "$RUN/server.log"; then
    echo "differential: the server logged a licence line" >&2
    return 1
  fi
}

stop_lab() {
  "$LAB" server stop >/dev/null 2>&1 || true
  "$LAB" tracker stop >/dev/null 2>&1 || true
}

status=0

if [[ -n "$KDX_BIN" ]]; then
  echo "differential: kdx-client suite"
  start_lab
  "$LAB" exec env \
    KDX_LAB_SERVER=127.0.0.1:10700 \
    KDX_LAB_TRACKER=127.0.0.1:10800 \
    KDX_LAB_PASSWORD="${KDX_ADMIN_PASSWORD:-openkdx-lab}" \
    KDX_CHALLENGE_TABLE="$KDX_CHALLENGE_TABLE" \
    "$KDX_BIN" --test-threads=1 || status=$?
  check_log || status=1
  stop_lab
fi

if [[ -n "$OPENKDX_BIN" ]]; then
  echo "differential: openkdx-client suite"
  start_lab
  "$LAB" exec env \
    OPENKDX_LAB_SERVER=127.0.0.1:10700 \
    OPENKDX_LAB_TRACKER=127.0.0.1:10800 \
    OPENKDX_LAB_PASSWORD="${KDX_ADMIN_PASSWORD:-openkdx-lab}" \
    OPENKDX_KDX_TABLE="$OPENKDX_KDX_TABLE" \
    "$OPENKDX_BIN" --test-threads=1 || status=$?
  check_log || status=1
  stop_lab
fi

[[ $status -eq 0 ]] && echo "differential: ok"
exit $status
