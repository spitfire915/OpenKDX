#!/usr/bin/env bash
# 150 Disconnect User with a ban: the victim's 151, the ban on reconnect, and a
# failed login (Proof status 1) for good measure.
source "$(dirname "$0")/../lib/scenario.sh"
# The inventory needs the wrong password too, to key that connection.
INVENTORY_EXTRA_ARGS="-p wrong-password"
scenario_begin disconnect-ban
use_cli
start_server
capture_begin
cli --as admin wrong-password login || true
step "A login with a wrong password: the server answers the proof with status 1 and closes (log: 'Login failed')."
cli_bg victim --as "" "" --nick victim login --stay 20
step "A guest session ('victim') logs in and stays."
cli users
cli server disconnect victim --ban 10 --message "bye"
step "Admin: users (the victim is listed), then server disconnect victim --ban 10 --message bye (150)."
cli_wait victim || step "The victim prints the 151 Disconnect Message (banned, 'bye') and exits with status 3; the admin gets a success Status for the 150."
cli_guest login
step "A guest login right after: the server's own ban list does not apply to 127.0.0.1 (reference sequences.md), so from the lab's loopback it succeeds; the ban itself is only visible as flag 2 in the 151."
capture_end
scenario_end
