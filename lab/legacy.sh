#!/usr/bin/env bash
# Legacy formats against the original programs (Phase 3), run by the lab CI
# job after lab/differential.sh:
#   1. lab/scenarios/legacy-samples.sh writes one snapshot of the server's
#      files per controlled change (Server 1.620 + Client 1.600 under Wine);
#   2. the legacy crate's tests/lab.rs round-trips every snapshot;
#   3. our writer adds an account and renames the server; the original
#      server is restarted on the rewritten files and the CLI logs in with
#      the new account and sees the new name.
#
#   lab/legacy.sh [KDX_SRC]    KDX_SRC defaults to ~/kdx-src. Its kdx-legacy
#                              crate's tests run and its legacytool writes the
#                              files when the checkout has the crate;
#                              openkdx-legacy's tests run while that crate exists.
#   lab/legacy.sh --supports-kdx-src   exits 0: the fork's lab job probes this
#
# The lock: with LAB_WAIT=1 (both workflows set it) a busy lab is waited for,
# up to 40 minutes; the scenarios take the lock themselves and honour LAB_WAIT.
set -euo pipefail

[[ "${1:-}" == --supports-kdx-src ]] && exit 0
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LAB="$HERE/kdx-lab"
RUN="${OPENKDX_LAB_RUN:-$HOME/lab/run}"
KDX_SRC="${1:-${OPENKDX_KDX_SRC:-$HOME/kdx-src}}"
export PATH="$HOME/.cargo/bin:$PATH"
source "${OPENKDX_LAB_ENV:-$HOME/haxial/lab.env}"

# legacy-samples.sh takes the lock itself; wait for (or check) a free lab first.
exec 9>"$RUN/lab.lock"
if [[ "${LAB_WAIT:-0}" == 1 ]]; then
  flock -w 2400 9 || { echo "legacy: the lab stayed busy for 40 min" >&2; exit 1; }
else
  flock -n 9 || { echo "legacy: lab busy"; exit 1; }
fi
exec 9>&-

if [[ -f "$KDX_SRC/crates/kdx-legacy/Cargo.toml" ]]; then
  echo "legacy: kdx-legacy at $(git -C "$KDX_SRC" rev-parse --short HEAD)"
  KDX_LEGACY_MANIFEST="$KDX_SRC/Cargo.toml"
else
  echo "legacy: no kdx-legacy in $KDX_SRC yet (Phase 3E); running openkdx-legacy only"
  KDX_LEGACY_MANIFEST=""
fi
HAVE_OPENKDX_LEGACY=0
[[ -d "$HERE/../crates/openkdx-legacy" ]] && HAVE_OPENKDX_LEGACY=1
[[ -n "$KDX_LEGACY_MANIFEST" || "$HAVE_OPENKDX_LEGACY" == 1 ]] || { echo "legacy: neither kdx-legacy in $KDX_SRC nor openkdx-legacy here" >&2; exit 1; }

FORCE=1 "$HERE/scenarios/legacy-samples.sh"

SNAP="$RUN/legacy"
# Nothing of Haxial's prose may end up in a snapshot we might commit: the
# stock welcome post lives in 30-news-stock only, which stays on the lab.
if grep -rlqa "Haxial Software" "$SNAP" --exclude-dir=30-news-stock; then echo "legacy: Haxial text in snapshots" >&2; exit 1; fi

cd "$HERE/.."
if [[ "$HAVE_OPENKDX_LEGACY" == 1 ]]; then
  OPENKDX_LAB_LEGACY_DIR="$SNAP" cargo test -q -p openkdx-legacy --test lab -- --test-threads=1 --skip lab_client
fi
if [[ -n "$KDX_LEGACY_MANIFEST" ]]; then
  KDX_LAB_LEGACY_DIR="$SNAP" cargo test -q --manifest-path "$KDX_LEGACY_MANIFEST" -p kdx-legacy --test lab -- --test-threads=1 --skip lab_client
fi

# --- Write with ours, read with theirs ---------------------------------------
# The files are written by kdx-legacy's legacytool when the checkout has it.
cargo build -q -p openkdx-cli
CLI="$PWD/target/debug/openkdx"
if [[ -n "$KDX_LEGACY_MANIFEST" ]]; then
  cargo build -q --manifest-path "$KDX_LEGACY_MANIFEST" -p kdx-legacy --example legacytool
  TOOL="$KDX_SRC/target/debug/examples/legacytool"
else
  cargo build -q -p openkdx-legacy --example legacytool
  TOOL="$PWD/target/debug/examples/legacytool"
fi
"$LAB" server stop >/dev/null 2>&1 || true
sleep 1
"$TOOL" add-account "$RUN/server" legacyuser secret 1
"$TOOL" set-name "$RUN/server" "Written by OpenKDX"
"$TOOL" post "$RUN/server" 1 "Posted by OpenKDX" "This post was appended to News.dat by openkdx-legacy."
"$TOOL" rewrite "$RUN/server"
test -f "$RUN/server/Accounts.dat.orig" && test -f "$RUN/server/KDXServer.stg.orig"
"$LAB" server start >/dev/null
for _ in $(seq 1 50); do grep -q "KDX Server started" "$RUN/server.log" && break; sleep 0.2; done
if [[ -z "${OPENKDX_KDX_TABLE:-}" ]]; then
  table_dir=$(cargo run -q -p openkdx-table -- path 2>/dev/null | sed -n 1p)
  OPENKDX_KDX_TABLE="$(base64 -w0 "$table_dir/kdx.bin")"
fi
out=$("$LAB" exec env OPENKDX_KDX_TABLE="$OPENKDX_KDX_TABLE" "$CLI" -s 127.0.0.1 -l legacyuser -p secret --json login 2>&1)
case "$out" in *'"Written by OpenKDX"'*) ;; *) echo "legacy: server did not take our files: $out" >&2; exit 1 ;; esac
"$LAB" exec env OPENKDX_KDX_TABLE="$OPENKDX_KDX_TABLE" "$CLI" -s 127.0.0.1 -l "$KDX_ADMIN_LOGIN" -p "$KDX_ADMIN_PASSWORD" accounts show legacyuser > "$RUN/legacy-show.out"; grep -q legacyuser "$RUN/legacy-show.out"
# The original must list the post we appended, and still read the news
# store it rewrites afterwards.
"$LAB" exec env OPENKDX_KDX_TABLE="$OPENKDX_KDX_TABLE" "$CLI" -s 127.0.0.1 -l "$KDX_ADMIN_LOGIN" -p "$KDX_ADMIN_PASSWORD" news list "General News" > "$RUN/legacy-news.out"
grep -q "Posted by OpenKDX" "$RUN/legacy-news.out" || { echo "legacy: server does not list our post" >&2; cat "$RUN/legacy-news.out" >&2; exit 1; }
"$LAB" exec env OPENKDX_KDX_TABLE="$OPENKDX_KDX_TABLE" "$CLI" -s 127.0.0.1 -l "$KDX_ADMIN_LOGIN" -p "$KDX_ADMIN_PASSWORD" news post "General News" --subject "After ours" --body "The original wrote this." > /dev/null
sleep 1
"$TOOL" dump "$RUN/server/News.dat" | grep -q "After ours" || { echo "legacy: cannot read the news store the original wrote after our post" >&2; exit 1; }
if grep -qi "licen\|damaged\|corrupt" "$RUN/server.log"; then echo "legacy: server log complains" >&2; tail -20 "$RUN/server.log" >&2; exit 1; fi
"$LAB" ns stop >/dev/null 2>&1 || true

# --- .hap: every file of the appearance pack round-trips (lab-only data) ----
if [[ -d "$HOME/haxial/legacy/appearances" ]]; then
  if [[ "$HAVE_OPENKDX_LEGACY" == 1 ]]; then
    OPENKDX_LAB_HAP_DIR="$HOME/haxial/legacy/appearances" cargo test -q -p openkdx-legacy --test hap -- --test-threads=1
  fi
  if [[ -n "$KDX_LEGACY_MANIFEST" ]]; then
    KDX_LAB_HAP_DIR="$HOME/haxial/legacy/appearances" cargo test -q --manifest-path "$KDX_LEGACY_MANIFEST" -p kdx-legacy --test hap -- --test-threads=1
  fi
else
  echo "legacy: appearance pack missing under ~/haxial/legacy/appearances; .hap pack test skipped" >&2
fi

# --- The client's file: samples, round trips, write-back --------------------
FORCE=1 "$HERE/scenarios/client-samples.sh"
if [[ "$HAVE_OPENKDX_LEGACY" == 1 ]]; then
  OPENKDX_LAB_LEGACY_DIR="$SNAP" cargo test -q -p openkdx-legacy --test lab -- --test-threads=1 lab_client
fi
if [[ -n "$KDX_LEGACY_MANIFEST" ]]; then
  KDX_LAB_LEGACY_DIR="$SNAP" cargo test -q --manifest-path "$KDX_LEGACY_MANIFEST" -p kdx-legacy --test lab -- --test-threads=1 lab_client
fi
"$LAB" ns stop >/dev/null 2>&1 || true
echo "legacy: ok"
