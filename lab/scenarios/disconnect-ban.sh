#!/usr/bin/env bash
# 150 Disconnect User with a ban: the victim's 151, the ban on reconnect, and a
# failed login (Proof status 1) for good measure.
source "$(dirname "$0")/../lib/scenario.sh"
scenario_begin disconnect-ban
use_cli
start_server
capture_begin
cli_guest --nick victim login --stay 1 >/dev/null || true
cli --as admin wrong-password login || true
step "A login with a wrong password: the server answers the proof with status 1 and closes (log: 'Login failed')."
cli_bg victim --as "" "" --nick victim login --stay 20
step "A guest session ('victim') logs in and stays."
cli users
cli server disconnect victim --ban 10 --message "bye"
step "Admin: users (the victim is listed), then server disconnect victim --ban 10 --message bye (150)."
cli_wait victim || step "The victim prints the 151 Disconnect Message (banned, 'bye') and exits with status 3."
cli_guest login || step "A guest login within the ban window is refused."
sleep 11
cli_guest login
step "After the ban (10 s) a guest login succeeds again."
capture_end
scenario_end
