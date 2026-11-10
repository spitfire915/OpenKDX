#!/usr/bin/env bash
# Inventory the legacy data directories under ~/haxial/legacy for docs/formats/inventory.md.
# Prints: relative path, size, sha256, first 8 bytes (hex). Files inside Bases/ are
# the user's shared files: listed by name and size only, no hash or bytes.
set -euo pipefail
ROOT="${1:-$HOME/haxial/legacy}"
cd "$ROOT"
echo "| Path | Size | SHA-256 (first 16) | First 8 bytes |"
echo "|---|---:|---|---|"
find . -type f | sort | while read -r f; do
  rel=${f#./}
  size=$(stat -c %s "$f")
  if [[ "$rel" == */Bases/* ]]; then
    echo "| \`$rel\` | $size | — | — |"
  else
    sha=$(sha256sum "$f" | cut -c1-16)
    head=$(head -c 8 "$f" | xxd -p)
    echo "| \`$rel\` | $size | \`$sha\` | \`$head\` |"
  fi
done
