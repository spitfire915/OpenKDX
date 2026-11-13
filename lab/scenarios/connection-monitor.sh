#!/usr/bin/env bash
# Connection monitor: 140, 142 → 143 while another session downloads, 141.
source "$(dirname "$0")/../lib/scenario.sh"
scenario_begin connection-monitor
use_cli
start_server
head -c 4194304 /dev/urandom > "$RUN/server/Bases/Default/four.bin"
step "Seed the base folder with four.bin (4 MiB)."
capture_begin
cli_bg downloader --nick downloader files get four.bin --to "$RUN/four.down"
step "A second admin session downloads four.bin on a transfer connection."
cli --nick monitor server monitor --for 6 --every 2
step "Admin: server monitor --for 6 --every 2 (140, then 142 → 143 four times: the sessions and the transfer with its byte counts, then 141)."
cli_wait downloader || true
step "The download completes."
capture_end
scenario_end
