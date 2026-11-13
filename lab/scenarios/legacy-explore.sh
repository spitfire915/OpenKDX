#!/usr/bin/env bash
# Exploration only (not a capture): open the Windows client's Server Settings
# window and screenshot it so the legacy-samples scenario can be scripted.
source "$(dirname "$0")/../lib/scenario.sh"
FORCE=1 scenario_begin legacy-explore
start_server
start_winclient
connect_to 127.0.0.1 "$KDX_ADMIN_LOGIN" "$KDX_ADMIN_PASSWORD"
sleep 3
palette_click commands
sleep 2
ui_shot legacy-explore-commands
X xdotool search --name "" 2>/dev/null | while read -r id; do n=$(X xdotool getwindowname "$id" 2>/dev/null); [[ -n "$n" ]] && echo "$id $(ui_geom "$id") $n"; done | sort -u
