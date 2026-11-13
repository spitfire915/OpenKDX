#!/usr/bin/env bash
# Installs Ghidra and a JDK under ~/ghidra on the lab machine and imports the
# Haxial binaries into one headless project. Lab only: the project and
# everything Ghidra derives from the binaries stay on that machine
# (design spec §5.3). Idempotent.
#
#   re/setup-ghidra.sh            install + import all binaries
#   re/setup-ghidra.sh --no-import
set -euo pipefail

GHIDRA_HOME="${GHIDRA_HOME:-$HOME/ghidra}"
BIN="${OPENKDX_LAB_BIN:-$HOME/haxial/bin}"
PROJECT_DIR="$GHIDRA_HOME/projects"
PROJECT="openkdx"

GHIDRA_URL="https://github.com/NationalSecurityAgency/ghidra/releases/download/Ghidra_12.2_build/ghidra_12.2_PUBLIC_20261016.zip"
GHIDRA_SHA="2c2cd776db3c1f6a097ba418922378ac3649cadd5e844a6f5f088ae628a7be67"
JDK_URL="https://github.com/adoptium/temurin25-binaries/releases/download/jdk-25.0.5%2B8/OpenJDK25U-jdk_x64_linux_hotspot_25.0.5_8.tar.gz"
JDK_SHA="68982529869257d0ee6b824ec5aa270ff8b118bb438670da0b97ebdab178d7fb"

fetch() { # URL SHA DEST
  if [[ -f "$3" ]] && echo "$2  $3" | sha256sum -c --quiet 2>/dev/null; then return; fi
  curl -fL --retry 3 -o "$3.part" "$1"
  echo "$2  $3.part" | sha256sum -c --quiet
  mv "$3.part" "$3"
}

mkdir -p "$GHIDRA_HOME" "$PROJECT_DIR"
cd "$GHIDRA_HOME"

if [[ ! -x jdk/bin/java ]]; then
  fetch "$JDK_URL" "$JDK_SHA" jdk.tar.gz
  rm -rf jdk jdk-tmp && mkdir jdk-tmp && tar -xzf jdk.tar.gz -C jdk-tmp --strip-components=1 && mv jdk-tmp jdk
fi
export JAVA_HOME="$GHIDRA_HOME/jdk" PATH="$GHIDRA_HOME/jdk/bin:$PATH"

if [[ ! -x ghidra/support/analyzeHeadless ]]; then
  fetch "$GHIDRA_URL" "$GHIDRA_SHA" ghidra.zip
  rm -rf ghidra ghidra_* && unzip -q ghidra.zip && mv ghidra_*_PUBLIC ghidra
fi
HEADLESS="$GHIDRA_HOME/ghidra/support/analyzeHeadless"

[[ "${1:-}" == "--no-import" ]] && exit 0

# Each binary becomes a program in the project; import once, analyse once.
import_one() { # PATH NAME
  if [[ -f "$PROJECT_DIR/$PROJECT.rep/idata/~index.dat" ]] && grep -q "$2" "$PROJECT_DIR/$PROJECT.rep/idata/~index.dat" 2>/dev/null; then
    echo "already imported: $2"; return
  fi
  "$HEADLESS" "$PROJECT_DIR" "$PROJECT" -import "$1" -overwrite -analysisTimeoutPerFile 3600 \
    -max-cpu 16 -log "$GHIDRA_HOME/import-$2.log" >/dev/null
  echo "imported: $2"
}

import_one "$BIN/KDXServer1620-Lnx/KDXServer1620-Lnx/KDXServer.lexe" KDXServer.lexe
import_one "$BIN/KDXTrackerServ1110-Lnx/KDXTrackerServ1110-Lnx/KDXTrackerServ.lexe" KDXTrackerServ.lexe
import_one "$BIN/KDXClient1600-Win/KDXClient1600-Win/KDXClient.exe" KDXClient.exe
import_one "$BIN/AppearanceEdit1240-Win/AppearanceEdit1240-Win/AppearanceEdit.exe" AppearanceEdit.exe
import_one "$BIN/KDXSoundListEdit1000-Win/KDXSoundListEdit1000-Win/KDXSoundListEdit.exe" KDXSoundListEdit.exe
echo "project: $PROJECT_DIR/$PROJECT"
