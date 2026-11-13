#!/usr/bin/env bash
# 154 Send Broadcast from the admin; 155 at a guest session and at the sender.
source "$(dirname "$0")/../lib/scenario.sh"
scenario_begin broadcast
use_cli
start_server
capture_begin
cli_bg guest --as "" "" --nick listener login --stay 8
step "A guest session ('listener') logs in and stays to print what arrives."
sleep 1
cli --nick announcer login --stay 3 &
ADMIN_PID=$!
sleep 1
cli --nick speaker server broadcast "hello all"
step "The admin sends: server broadcast 'hello all' (154)."
wait $ADMIN_PID || true
cli_wait guest || true
step "Both the guest and the other admin session print the 155 Broadcast; so does nobody else on the sending session, which closes at once."
capture_end
scenario_end
