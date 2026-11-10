#!/usr/bin/env bash
# Win32 Client 1.600 logs in as the administrator to Server 1.620.
source "$(dirname "$0")/../lib/scenario.sh"
scenario_begin first-connect
start_server
start_winclient
capture_begin
connect_to 127.0.0.1 "$KDX_ADMIN_LOGIN" "$KDX_ADMIN_PASSWORD"
sleep 5
step "Wait 5 s connected (idle), then stop capturing."
capture_end
scenario_end
