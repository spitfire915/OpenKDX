#!/usr/bin/env bash
# Legacy formats against the original programs (Phase 3), run by the lab CI
# job after lab/differential.sh:
#   1. lab/scenarios/legacy-samples.sh writes one snapshot of the server's
#      files per controlled change (Server 1.620 + Client 1.600 under Wine);
#   2. crates/openkdx-legacy/tests/lab.rs round-trips every snapshot;
#   3. our writer adds an account and renames the server; the original
#      server is restarted on the rewritten files and the CLI logs in with
#      the new account and sees the new name.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LAB="$HERE/kdx-lab"
RUN="${OPENKDX_LAB_RUN:-$HOME/lab/run}"
export PATH="$HOME/.cargo/bin:$PATH"
source "${OPENKDX_LAB_ENV:-$HOME/haxial/lab.env}"

# legacy-samples.sh takes the lock itself; make sure nothing holds it first.
exec 9>"$RUN/lab.lock"
flock -n 9 || { echo "legacy: lab busy"; exit 1; }
exec 9>&-

FORCE=1 "$HERE/scenarios/legacy-samples.sh"

SNAP="$RUN/legacy"
# Nothing of Haxial's prose may end up in a snapshot we might commit.
if grep -rqa "Haxial Software" "$SNAP"; then echo "legacy: Haxial text in snapshots" >&2; exit 1; fi

cd "$HERE/.."
OPENKDX_LAB_LEGACY_DIR="$SNAP" cargo test -q -p openkdx-legacy --test lab -- --test-threads=1

# --- Write with ours, read with theirs ---------------------------------------
cargo build -q -p openkdx-legacy --example legacytool -p openkdx-cli
TOOL="$PWD/target/debug/examples/legacytool"
CLI="$PWD/target/debug/openkdx"
"$LAB" server stop >/dev/null 2>&1 || true
sleep 1
"$TOOL" add-account "$RUN/server" legacyuser secret 1
"$TOOL" set-name "$RUN/server" "Written by OpenKDX"
"$TOOL" rewrite "$RUN/server"
test -f "$RUN/server/Accounts.dat.orig" && test -f "$RUN/server/KDXServer.stg.orig"
"$LAB" server start >/dev/null
for _ in $(seq 1 50); do grep -q "KDX Server started" "$RUN/server.log" && break; sleep 0.2; done
if [[ -z "${OPENKDX_KDX_TABLE:-}" ]]; then
  OPENKDX_KDX_TABLE="$(base64 -w0 "$(cargo run -q -p openkdx-table -- path | head -1)/kdx.bin")"
fi
out=$("$LAB" exec env OPENKDX_KDX_TABLE="$OPENKDX_KDX_TABLE" "$CLI" -s 127.0.0.1 -l legacyuser -p secret --json login)
echo "$out" | grep -q '"Written by OpenKDX"' || { echo "legacy: server did not take our files: $out" >&2; exit 1; }
"$LAB" exec env OPENKDX_KDX_TABLE="$OPENKDX_KDX_TABLE" "$CLI" -s 127.0.0.1 -l "$KDX_ADMIN_LOGIN" -p "$KDX_ADMIN_PASSWORD" accounts show legacyuser > "$RUN/legacy-show.out"; grep -q legacyuser "$RUN/legacy-show.out"
if grep -qi "licen\|damaged\|corrupt" "$RUN/server.log"; then echo "legacy: server log complains" >&2; tail -20 "$RUN/server.log" >&2; exit 1; fi
"$LAB" ns stop >/dev/null 2>&1 || true
echo "legacy: ok"
