#!/usr/bin/env bash
# Accounts and classes over the wire: 502–513 and 515.
source "$(dirname "$0")/../lib/scenario.sh"
scenario_begin accounts-classes
use_cli
start_server
capture_begin
cli_run <<'CMDS'
classes list
classes show 1
accounts list
accounts create tester --new-password t --name Tester --class 1 --privileges can-login,download
accounts show tester
accounts set tester --name "Tester 2"
accounts show tester
accounts delete tester
accounts list
CMDS
step "One admin session runs: classes list (505/506), classes show 1 (502/503), accounts list (512/513), accounts create tester (507 → Status 0x1fb), accounts show tester (509/510), accounts set tester --name 'Tester 2' (511 → Status 0x1ff), accounts show, accounts delete tester (508 → 515), accounts list."
capture_end
scenario_end
