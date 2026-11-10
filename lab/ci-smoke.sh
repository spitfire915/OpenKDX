#!/usr/bin/env bash
# The `lab` CI job: prove the sandbox works and the original KDX Server 1.620
# runs in it and answers on the wire. Runs on the self-hosted lab runner as
# the openkdx user. Exit non-zero on any failure.
set -euo pipefail

HERE=$(cd "$(dirname "$0")" && pwd)
LAB="$HERE/kdx-lab"
RUN="${OPENKDX_LAB_RUN:-$HOME/lab/run}"

cleanup() { "$LAB" ns stop >/dev/null 2>&1 || true; }
trap cleanup EXIT

# Refuse to trample a human's lab session (Review Focus 5).
exec 9>"$RUN/lab.lock"
flock -n 9 || { echo "smoke: the lab is in use (lock $RUN/lab.lock); not running" >&2; exit 1; }
"$LAB" ns status | grep -q running && { echo "smoke: a lab session is running; not running" >&2; exit 1; }

[[ -x "$HOME/haxial/bin/KDXServer1620-Lnx/KDXServer1620-Lnx/KDXServer.lexe" ]] \
  || { echo "smoke: Haxial binaries missing; run lab/fetch-binaries.sh" >&2; exit 1; }

"$LAB" selftest

# Fresh server state every run so first-run setup is exercised too.
"$LAB" ns stop >/dev/null 2>&1 || true
"$LAB" reset server >/dev/null 2>&1 || true
"$LAB" ns start
"$LAB" server start

for _ in $(seq 1 50); do
  grep -q "KDX Server started" "$RUN/server.log" && break
  sleep 0.2
done
grep -q "KDX Server started" "$RUN/server.log" || { "$LAB" server log; echo "smoke: server did not start" >&2; exit 1; }

# A 64-byte junk hello must be rejected with the known error.
"$LAB" exec sh -c 'head -c 64 /dev/zero | tr "\0" X | timeout 3 nc -w2 127.0.0.1 10700 >/dev/null' || true

for _ in $(seq 1 25); do
  grep -q "The network protocol is unknown. (ID 4/113 @ TRN941)" "$RUN/server.log" && break
  sleep 0.2
done
grep -q "The network protocol is unknown. (ID 4/113 @ TRN941)" "$RUN/server.log" \
  || { "$LAB" server log; echo "smoke: server did not reject junk hello" >&2; exit 1; }

"$LAB" server stop
echo "smoke: ok"
