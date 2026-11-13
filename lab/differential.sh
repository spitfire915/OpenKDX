#!/usr/bin/env bash
# Differential tests: the OpenKDX client library against the original KDX
# Server 1.620 and Tracker 1.110 in the sandbox (crates/openkdx-client/tests/lab.rs).
# Runs on the lab machine, as the openkdx user; respects the lab lock as
# lab/ci-smoke.sh does. Exit non-zero on any failure.
set -euo pipefail

HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/.." && pwd)
LAB="$HERE/kdx-lab"
RUN="${OPENKDX_LAB_RUN:-$HOME/lab/run}"
export PATH="$HOME/.cargo/bin:$PATH"

cleanup() { "$LAB" ns stop >/dev/null 2>&1 || true; }

exec 9>"$RUN/lab.lock"
flock -n 9 || { echo "differential: the lab is in use (lock $RUN/lab.lock); not running" >&2; exit 1; }
"$LAB" ns status | grep -q running && { echo "differential: a lab session is running; not running" >&2; exit 1; }
# Only after the lock: a refused run must not tear down someone else's namespace.
trap cleanup EXIT

# The challenge table, from the cache or the server binary (as ci.yml does).
cargo run -q -p openkdx-table --manifest-path "$REPO/Cargo.toml" -- path | grep -q "kdx.bin: present" \
  || cargo run -q -p openkdx-table --manifest-path "$REPO/Cargo.toml" -- import \
       "$HOME/haxial/bin/KDXServer1620-Lnx/KDXServer1620-Lnx/KDXServer.lexe"
TABLE_DIR=$(cargo run -q -p openkdx-table --manifest-path "$REPO/Cargo.toml" -- path 2>/dev/null | sed -n 1p)
OPENKDX_KDX_TABLE="$(base64 -w0 "$TABLE_DIR/kdx.bin")"
export OPENKDX_KDX_TABLE

# Build outside the namespace (no network inside it).
cargo build -q --manifest-path "$REPO/Cargo.toml" -p openkdx-client --tests
TEST_BIN=$(cargo test -q --manifest-path "$REPO/Cargo.toml" -p openkdx-client --test lab --no-run --message-format=json 2>/dev/null \
  | python3 -c 'import json,sys
for line in sys.stdin:
    try: m=json.loads(line)
    except ValueError: continue
    if m.get("reason")=="compiler-artifact" and m.get("executable") and m["target"]["name"]=="lab":
        print(m["executable"])')
[[ -x "$TEST_BIN" ]] || { echo "differential: test binary not found" >&2; exit 1; }

# Fresh server and tracker; the server registers with the tracker on startup.
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

status=0
"$LAB" exec env \
  OPENKDX_LAB_SERVER=127.0.0.1:10700 \
  OPENKDX_LAB_TRACKER=127.0.0.1:10800 \
  OPENKDX_LAB_PASSWORD="${KDX_ADMIN_PASSWORD:-openkdx-lab}" \
  OPENKDX_KDX_TABLE="$OPENKDX_KDX_TABLE" \
  "$TEST_BIN" --test-threads=1 || status=$?

echo "--- server log (tail)"
sed 's/\x1b\[[0-9;]*[A-Za-z]//g' "$RUN/server.log" | tail -40

# The client presents itself as a registered Client 1.600 (design §9): the
# server must not log a licence reminder.
if grep -qi "licen" "$RUN/server.log"; then
  echo "differential: the server logged a licence line" >&2
  status=1
fi

"$LAB" server stop >/dev/null 2>&1 || true
"$LAB" tracker stop >/dev/null 2>&1 || true
[[ $status -eq 0 ]] && echo "differential: ok"
exit $status
