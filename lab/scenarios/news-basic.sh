#!/usr/bin/env bash
# Admin lists newsgroups, reads General News, posts a message.
source "$(dirname "$0")/../lib/scenario.sh"
scenario_begin news-basic
start_server
start_winclient
capture_begin
connect_to 127.0.0.1 "$KDX_ADMIN_LOGIN" "$KDX_ADMIN_PASSWORD"
palette_click news
ui_wait "KDX News" 10
N=$(ui_win "KDX News"); ui_focus "$N"
ui_click "$N" 23 58; sleep 1.5
step "Open News; expand the server node (shows 'General News', count 1)."
ui_click "$N" 115 77; sleep 2
step "Click 'General News' to list and read its messages in the lower pane."
ui_click "$N" 355 426; sleep 1.5
M=$(ui_win "Message To: General News"); ui_focus "$M"
ui_click "$M" 240 84; ui_type "Lab post"; ui_click "$M" 200 200; ui_type "Posted from the lab scenario."
ui_click "$M" 308 321; sleep 2
step "Click 'Post Message…'; subject 'Lab post', body 'Posted from the lab scenario.'; press Send."
ui_click "$N" 115 77; sleep 2
step "Click 'General News' again to re-read (count should now be 2)."
capture_end
scenario_end
