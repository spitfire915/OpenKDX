#!/usr/bin/env bash
# Download, verify, and unpack the Haxial archives listed in lab/MANIFEST.txt
# into ~/haxial/bin, then extract the challenge tables into ~/haxial/tables.
# Runs as the unprivileged lab user. Idempotent: verified files are skipped.
set -euo pipefail

HERE=$(cd "$(dirname "$0")" && pwd)
MANIFEST="$HERE/MANIFEST.txt"
BIN="${OPENKDX_LAB_BIN:-$HOME/haxial/bin}"
TABLES="${OPENKDX_LAB_TABLES:-$HOME/haxial/tables}"
mkdir -p "$BIN" "$TABLES"

sha() { sha256sum "$1" | cut -d' ' -f1; }

fail=0
while read -r want name url; do
  [[ -z "$want" || "$want" == \#* ]] && continue
  dest="$BIN/$name"
  if [[ -f "$dest" && "$(sha "$dest")" == "$want" ]]; then
    echo "ok        $name (cached)"
  else
    tmp="$dest.part"
    if ! curl -fsSL -A "OpenKDX-lab/0.0 (+https://github.com/spitfire915/OpenKDX)" -o "$tmp" "$url"; then
      echo "FAIL      $name: download failed from $url" >&2; rm -f "$tmp"; fail=1; continue
    fi
    got=$(sha "$tmp")
    if [[ "$got" != "$want" ]]; then
      echo "FAIL      $name: sha256 $got != $want" >&2; rm -f "$tmp"; fail=1; continue
    fi
    mv "$tmp" "$dest"
    echo "ok        $name (downloaded)"
  fi
  stem="${name%.zip}"
  if [[ ! -d "$BIN/$stem" ]]; then
    mkdir -p "$BIN/$stem"
    unzip -q -o "$dest" -d "$BIN/$stem"
    find "$BIN/$stem" -name '*.lexe' -exec chmod +x {} +
    echo "unpacked  $stem/"
  fi
done < "$MANIFEST"

[[ $fail -eq 0 ]] || { echo "some archives failed" >&2; exit 1; }

# Challenge tables, via the Rust tool if available, else leave for later.
if command -v cargo >/dev/null 2>&1 && [[ -f "$HERE/../Cargo.toml" ]]; then
  (cd "$HERE/.." && cargo run -q -p openkdx-table -- --cache "$TABLES" import "$BIN/KDXTrackerServ1110-Win.zip")
  (cd "$HERE/.." && cargo run -q -p openkdx-table -- --cache "$TABLES" import "$BIN/KDXClient1600-Win.zip")
  (cd "$HERE/.." && cargo run -q -p openkdx-table -- --cache "$TABLES" verify)
else
  echo "cargo not found; run 'openkdx-table --cache $TABLES import' later" >&2
fi
