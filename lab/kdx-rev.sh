#!/usr/bin/env bash
# The kdx revision OpenKDX pins, and the lab's checkout of it.
#
#   lab/kdx-rev.sh           prints the rev of kdx-proto from Cargo.lock
#   lab/kdx-rev.sh sync      checks ~/kdx-src (or $OPENKDX_KDX_SRC) out at it,
#                            cloning https://github.com/spitfire915/kdx if missing
#   lab/kdx-rev.sh sync REF  the same, at an explicit ref (a branch under test)
set -euo pipefail

HERE=$(cd "$(dirname "$0")" && pwd)
LOCK="$HERE/../Cargo.lock"
KDX_SRC="${OPENKDX_KDX_SRC:-$HOME/kdx-src}"
KDX_URL="${OPENKDX_KDX_URL:-https://github.com/spitfire915/kdx}"

pinned_rev() {
  # The source line of the kdx-proto package: git+URL?rev=SHA#SHA.
  awk '
    /^name = "kdx-proto"$/ { found = 1; next }
    found && /^source = / {
      if (match($0, /rev=[0-9a-f]+/)) { print substr($0, RSTART + 4, RLENGTH - 4); exit }
    }
    /^\[\[package\]\]/ { found = 0 }
  ' "$LOCK"
}

rev=$(pinned_rev)
[[ -n "$rev" ]] || { echo "kdx-rev: no pinned kdx-proto rev in $LOCK" >&2; exit 1; }

case "${1:-}" in
  "")
    echo "$rev"
    ;;
  sync)
    ref="${2:-$rev}"
    if [[ ! -d "$KDX_SRC/.git" ]]; then
      git clone -q "$KDX_URL" "$KDX_SRC"
    fi
    git -C "$KDX_SRC" fetch -q origin
    if [[ "$ref" == "$rev" ]]; then
      git -C "$KDX_SRC" checkout -q --detach "$rev"
    else
      # A branch or tag: take origin's copy of it.
      git -C "$KDX_SRC" checkout -q --detach "origin/$ref" 2>/dev/null \
        || git -C "$KDX_SRC" checkout -q --detach "$ref"
    fi
    echo "kdx-rev: $KDX_SRC at $(git -C "$KDX_SRC" rev-parse --short HEAD) ($ref)"
    ;;
  *)
    echo "usage: lab/kdx-rev.sh [sync [REF]]" >&2
    exit 2
    ;;
esac
