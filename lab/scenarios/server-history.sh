#!/usr/bin/env bash
# Server history: 180 → 181 with the records so far, then 183 as a guest logs
# in and out, then 182.
source "$(dirname "$0")/../lib/scenario.sh"
scenario_begin server-history
use_cli
start_server
capture_begin
cli_bg historian --nick historian server history --follow 8
step "Admin: server history --follow 8 (180 → 181 lists the Server Started and Login records; 183 follows for each new record)."
sleep 1
cli_guest --nick passer-by login
step "A guest logs in and out: the historian prints 183 Login and 183 Disconnected records."
cli_guest --nick poster news post 1 --subject hello --body "from the lab" || true
step "A guest posts a news message: another 183."
cli_wait historian || true
step "The historian leaves (182)."
capture_end
scenario_end
