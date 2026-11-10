#!/usr/bin/env bash
# Two clients: Linux beta 5 as guest, Windows 1.600 as admin. One private
# message each way.
PROGRAMS="KDX Server 1.620 (Linux), KDX Client 1.600 (Windows under Wine 6.0.3), KDX Client 1.620 beta 5 (Linux)"
source "$(dirname "$0")/../lib/scenario.sh"
scenario_begin pm-basic
start_server
start_lnxclient lnxuser
start_winclient labuser
capture_begin
use_client lnx
connect_to 127.0.0.1 "" ""
use_client win
connect_to 127.0.0.1 "$KDX_ADMIN_LOGIN" "$KDX_ADMIN_PASSWORD"
use_client win
palette_click userlist
U=$(ui_win "KDX User List" $WINCLASS); ui_focus "$U"
ui_click "$U" 15 60; sleep 1.5
step "Admin (Windows client) opens User List and expands the server: lnxuser (guest, black bar) and labuser (admin, red bar)."
ui_rclick "$U" 150 86; sleep 1; ui_click "$U" 225 97; sleep 1.5
M=$(ui_win "Message To: lnxuser" $WINCLASS); ui_focus "$M"
ui_type "hi from admin"; ui_click "$M" 258 334; sleep 3
step "Right-click lnxuser → Send Message…; type 'hi from admin'; Send. The guest's Messages WonderLight lights up."
use_client lnx
palette_click messages; sleep 2
P=$(ui_win "KDX Private Messages" $LNXCLASS); ui_focus "$P"; X xdotool windowmove "$P" 300 400; sleep 0.5
ui_click "$P" 80 50; sleep 0.5
ui_click "$P" 380 262; ui_type "hello back"; ui_key Return; sleep 3
step "Guest (Linux client) opens Messages, selects labuser, types 'hello back' and presses Return."
capture_end
scenario_end
