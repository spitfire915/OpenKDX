#!/usr/bin/env bash
# Runs a Ghidra script from re/scripts/ against one program of the lab
# project, headless, and prints the script's output.
#
#   re/ghidra-run.sh PROGRAM SCRIPT [ARGS…]
#   re/ghidra-run.sh KDXServer.lexe StringsXrefs.java Accounts.dat KDXServer.stg
set -euo pipefail
GHIDRA_HOME="${GHIDRA_HOME:-$HOME/ghidra}"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROGRAM=$1; SCRIPT=$2; shift 2
export JAVA_HOME="$GHIDRA_HOME/jdk" PATH="$GHIDRA_HOME/jdk/bin:$PATH"
OUT=$(mktemp)
"$GHIDRA_HOME/ghidra/support/analyzeHeadless" "$GHIDRA_HOME/projects" openkdx \
  -process "$PROGRAM" -noanalysis -readOnly -scriptPath "$HERE/scripts" \
  -postScript "$SCRIPT" "$@" -scriptlog "$OUT" >/dev/null 2>&1 || true
# Script output lines are prefixed by Ghidra's logger; keep only our marker.
sed -n 's/^.*@@ //p' "$OUT"
rm -f "$OUT"
