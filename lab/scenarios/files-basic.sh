#!/usr/bin/env bash
# Admin lists the base folder and downloads a 3-byte file.
# (Upload is not captured here: the Windows client's own file chooser shows no
# volumes under Wine. The Rust client will capture uploads in Phase 2.)
source "$(dirname "$0")/../lib/scenario.sh"
scenario_begin files-basic
start_server
printf 'abc' > "$RUN/server/Bases/Default/tiny.txt"
step "Seed the server's base folder with tiny.txt (3 bytes, 'abc')."
start_winclient
capture_begin
connect_to 127.0.0.1 "$KDX_ADMIN_LOGIN" "$KDX_ADMIN_PASSWORD"
palette_click files
ui_wait "Files @ 127.0.0.1:10700" 10
F=$(ui_win "Files @ 127.0.0.1:10700"); ui_focus "$F"; sleep 1
step "Open File Browser: the base lists 'Admin Dropbox' [DB], tiny.txt (3), 'Uploads' [UL]."
ui_rclick "$F" 290 123; sleep 1
ui_click "$F" 344 166; sleep 4
step "Right-click tiny.txt → Download. The client opens a second connection for the transfer."
[[ -s "$RUN/winclient/Downloads/tiny.txt" ]] && step "Verified: Downloads/tiny.txt arrived on the client ($(cat "$RUN/winclient/Downloads/tiny.txt"))."
ui_click "$F" 118 35; sleep 2
step "Click Refresh to list the base again."
capture_end
scenario_end
