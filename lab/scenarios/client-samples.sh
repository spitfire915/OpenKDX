#!/usr/bin/env bash
# Client-side samples: KDX.stg from KDX Client 1.600 (Windows, under Wine),
# one change per clean exit. Output: $RUN/legacy/6N-*/KDX.stg.
source "$(dirname "$0")/../lib/scenario.sh"
source "$(dirname "$0")/../lib/ui.sh"
OUT="$RUN/legacy"; mkdir -p "$OUT"
W="$RUN/winclient"

snap() { mkdir -p "$OUT/$1"; sleep 1; cp "$W/KDX.stg" "$OUT/$1/"; echo "snap $1: KDX.stg=$(stat -c %s "$W/KDX.stg")"; }
exit_client() {
  palette_click exit; sleep 2
  for _ in $(seq 1 30); do "$LAB" winclient status 2>/dev/null | grep -q running || break; sleep 0.5; done
  "$LAB" winclient stop >/dev/null 2>&1 || true
  sleep 1
}
restart_client() {
  "$LAB" winclient start >/dev/null
  PALETTE=$(ui_wait_sized KDX 164x310 30 $WINCLASS) || { echo "no palette after restart" >&2; exit 1; }
  CLASS=$WINCLASS; sleep 1
}
press_nf() { X xdotool mousemove --window "$1" "$2" "$3"; sleep 0.2; X xdotool mousedown 1; sleep 0.2; X xdotool mouseup 1; sleep 0.6; }
press() { ui_focus "$1"; press_nf "$@"; }
tap() { X xdotool mousemove --window "$1" "$2" "$3"; sleep 0.3; X xdotool click 1; sleep 0.8; }
open_settings() {
  ui_focus "$PALETTE"; ui_click "$PALETTE" 80 79; sleep 1
  X xdotool mousemove 80 331; sleep 0.5; X xdotool mousedown 1; sleep 0.3; X xdotool mouseup 1
  ui_wait "KDX Settings" 10 "$CLASS"; SET=$(ui_win "KDX Settings" "$CLASS" || true)
}
settings_save() { tap "$SET" 406 39; sleep 1.5; }
replace_field() { press "$1" "$2" "$3"; ui_key End; for _ in $(seq 1 "${5:-12}"); do ui_key BackSpace; done; ui_type "$4"; }

FORCE=1 scenario_begin client-samples
start_winclient
exit_client;                                                      snap 60-client-fresh
restart_client; open_settings; replace_field "$SET" 290 89 "legacynick"; settings_save
exit_client;                                                      snap 61-client-nick
restart_client; open_settings; press "$SET" 290 140; ui_type "A client description"; settings_save
exit_client;                                                      snap 62-client-description
restart_client
palette_click addressbook; ui_wait "KDX Address Book" 10 "$CLASS"; AB=$(ui_win "KDX Address Book" "$CLASS" || true)
ui_focus "$AB"; X xdotool mousemove --window "$AB" 150 150; sleep 0.3; X xdotool click 3; sleep 1
X xdotool mousemove_relative 40 10; sleep 0.3; X xdotool mousedown 1; sleep 0.2; X xdotool mouseup 1
ui_wait "New Address Book Item" 10 "$CLASS"; N=$(ui_win "New Address Book Item" "$CLASS" || true)
press "$N" 245 70; ui_type "Lab"; press "$N" 245 100; ui_type "127.0.0.1"; press "$N" 245 130; ui_type "admin"
press "$N" 245 160; ui_type "openkdx-lab"; press "$N" 245 190; ui_type "The lab server"; tap "$N" 313 434; sleep 1
ui_shot client-samples-addressbook
exit_client;                                                      snap 63-client-address-book
restart_client
palette_click addressbook; AB=$(ui_win "KDX Address Book" "$CLASS" || true)
ui_focus "$AB"; X xdotool mousemove --window "$AB" 150 150; sleep 0.3; X xdotool click 3; sleep 1
X xdotool mousemove_relative 40 10; sleep 0.3; X xdotool mousedown 1; sleep 0.2; X xdotool mouseup 1
ui_wait "New Address Book Item" 10 "$CLASS"; N=$(ui_win "New Address Book Item" "$CLASS" || true)
press "$N" 245 70; ui_type "Second"; press "$N" 245 100; ui_type "10.0.0.2"; tap "$N" 313 434; sleep 1
exit_client;                                                      snap 64-client-address-book-2
echo "client snapshots done"

# --- Write with ours, read with theirs ---------------------------------------
# legacytool rewrites the identity name; the client must start on the file
# and show the name in the Settings window (checked by a screenshot and by
# the server's log line "Set Name: <…>").
export PATH="$HOME/.cargo/bin:$PATH"
cargo build -q -p openkdx-legacy --example legacytool --manifest-path "$HERE/../Cargo.toml"
"$HERE/../target/debug/examples/legacytool" set-nick "$W/KDX.stg" openkdxwrote
start_server
restart_client
connect_to 127.0.0.1 "$KDX_ADMIN_LOGIN" "$KDX_ADMIN_PASSWORD"; sleep 3
ui_shot client-samples-written
if grep -q "Set Name: <openkdxwrote>" "$RUN/server.log"; then echo "client-written: ok"; else echo "client-written: FAILED (no Set Name <openkdxwrote> in server log)"; exit 1; fi
exit_client
"$LAB" server stop >/dev/null 2>&1 || true
