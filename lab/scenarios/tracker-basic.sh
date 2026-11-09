#!/usr/bin/env bash
# Server registers with Tracker 1.110 over UDP; client browses the tracker.
PROGRAMS="KDX Server 1.620 (Linux), KDX Tracker 1.110 (Linux), KDX Client 1.600 (Windows under Wine 6.0.3)"
source "$(dirname "$0")/../lib/scenario.sh"
scenario_begin tracker-basic
start_tracker
start_server
start_winclient
connect_to 127.0.0.1 "$KDX_ADMIN_LOGIN" "$KDX_ADMIN_PASSWORD"
# Commands → <server> → Server Settings (20th row of the submenu).
ui_commands_server_item "$PALETTE" 20
ui_wait "Server Settings @ 127.0.0.1:10700" 10 $WINCLASS
S=$(ui_win "Server Settings @ 127.0.0.1:10700" $WINCLASS)
X xdotool windowmove "$S" 500 450; sleep 0.5; ui_focus "$S"
step "Commands → 127.0.0.1:10700 → Server Settings."
ui_popup "$S" 219 37 6
step "Switch the panel popup to 'Trackers'."
for _ in 1 2 3; do
  ui_focus "$S"; ui_click "$S" 84 80; sleep 1.5
  ui_wait "Edit Tracker" 3 $WINCLASS && break
done
E=$(ui_win "Edit Tracker" $WINCLASS); ui_focus "$E"
ui_type "Lab tracker"; ui_click "$E" 260 136; ui_type "127.0.0.1"; ui_click "$E" 260 260; ui_type "General"
ui_click "$E" 368 341; sleep 1.5
step "Add Tracker…: name 'Lab tracker', address 127.0.0.1, group 'General'; Save."
ui_click "$S" 184 81; sleep 0.5; ui_click "$S" 378 39; sleep 3
step "Tick 'Register with Trackers'; Save the server settings."
"$LAB" winclient stop >/dev/null; "$LAB" reset winclient >/dev/null
step "Quit the client."
capture_begin
"$LAB" server stop >/dev/null; sleep 1; "$LAB" server start >/dev/null; sleep 5
step "Restart the server: it registers with the tracker on startup (one UDP datagram to 10800)."
start_winclient trackeruser
palette_click connect
ui_wait "KDX Connect" 8 $WINCLASS
C=$(ui_win "KDX Connect" $WINCLASS); ui_focus "$C"
ui_popup "$C" 140 43 4
ui_click "$C" 240 97; ui_type 127.0.0.1; ui_click "$C" 312 190; sleep 5
step "Fresh client: Connect… with Type 'Tracker', address 127.0.0.1. The Tracker window lists the groups; 'General' has count 1."
T=$(ui_win "Tracker: 127.0.0.1:10800" $WINCLASS); ui_focus "$T"
ui_click "$T" 110 129; sleep 3
step "Click 'General': the server list shows 'New Server'."
capture_end
scenario_end
