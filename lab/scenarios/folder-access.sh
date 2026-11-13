#!/usr/bin/env bash
# Folder access items: 1100–1107, and a folder bound to one (402).
source "$(dirname "$0")/../lib/scenario.sh"
scenario_begin folder-access
use_cli
start_server
capture_begin
cli_run <<'CMDS'
folder-access list
folder-access show "UL"
folder-access create Pictures --icon pictures --description pics
folder-access show Pictures
files mkdir "" Pictures --access Pictures
files ls -L
folder-access delete Pictures
folder-access list
CMDS
step "One admin session runs: folder-access list (1106/1107), show UL (1103/1104), create Pictures (1100 → Status 0x44c), show Pictures, files mkdir '' Pictures --access Pictures (402 → Status 0x192), files ls (400/401 shows 'Pictures [Pictures]'), folder-access delete Pictures (1101 → 1102), list."
capture_end
scenario_end
