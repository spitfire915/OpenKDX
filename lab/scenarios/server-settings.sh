#!/usr/bin/env bash
# Server settings: 170/171, 172 with a new name, and the name in a fresh 105.
source "$(dirname "$0")/../lib/scenario.sh"
scenario_begin server-settings
use_cli
start_server
capture_begin
cli_run <<'CMDS'
server settings
server set-name "OpenKDX Lab"
server settings
CMDS
step "Admin: server settings (170/171), server set-name 'OpenKDX Lab' (170/171 then 172 with section 0's name and length byte changed → Status), server settings again."
cli login
step "A second login: 105 Server Info now names the server 'OpenKDX Lab'."
cli server set-name "New Server"
step "Restore the name."
capture_end
scenario_end
