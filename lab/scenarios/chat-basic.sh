#!/usr/bin/env bash
# Admin joins Public Chat, sends a line and a /me action, sets the topic.
source "$(dirname "$0")/../lib/scenario.sh"
scenario_begin chat-basic
start_server
start_winclient
connect_to 127.0.0.1 "$KDX_ADMIN_LOGIN" "$KDX_ADMIN_PASSWORD"
capture_begin
palette_click chat
ui_wait "KDX Chat" 10
C=$(ui_win "KDX Chat"); ui_focus "$C"
ui_click "$C" 22 90;   sleep 1          # expand the server node
step "Open Chat; expand the server in the Chat List (shows Admin Lounge, Public Chat, Voice Chat)."
ui_click "$C" 120 128; sleep 2          # Public Chat row
C=$(ui_win "Chat: 127.0.0.1:10700"); ui_focus "$C"
step "Click 'Public Chat' to join it; the window becomes 'Chat: 127.0.0.1:10700' with a tab per server."
ui_click "$C" 150 292; ui_type "hello from the lab"; ui_key Return; sleep 1
step "Type 'hello from the lab' in the input box and press Return."
ui_type "/me waves"; ui_key Return; sleep 1
step "Type '/me waves' and press Return (action format)."
ui_click "$C" 38 331; sleep 1.5
T=$(ui_win "Public Chat @ 127.0.0.1:10700")
ui_focus "$T"; ui_click "$T" 180 90; ui_type "lab topic"; ui_click "$T" 320 155; sleep 1.5
step "Click 'Topic:'; in the Set topic dialog type 'lab topic' and press Set."
sleep 2
capture_end
scenario_end
