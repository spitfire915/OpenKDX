#!/usr/bin/env bash
# The OpenKDX client uploads a 256 KiB file to the Uploads folder, loses the
# connection part-way, resumes, then downloads it back: 415–418 and 417 data.
source "$(dirname "$0")/../lib/scenario.sh"
scenario_begin upload-resume
use_cli
start_server
head -c 262144 /dev/urandom > "$RUN/big.bin"
step "Make big.bin: 256 KiB of random bytes on the client side."
capture_begin
cli files put "$RUN/big.bin" "Uploads [UL]" --stop-after 150000 || true
step "openkdx files put big.bin 'Uploads [UL]' --stop-after 150000: 415, 416 (offset 0), a 417 header for 262144 bytes, then the client drops the connection after 150000 bytes."
sleep 1
cli files ls "Uploads [UL]" -L
step "openkdx files ls: the server keeps the partial as big.bin.prt."
cli files put "$RUN/big.bin" "Uploads [UL]" --resume
step "openkdx files put --resume: 416 offers the offset the server holds (a multiple of 0x1000); the client sends the rest under one 417 header, an empty 417 for the resource fork, and gets 418."
cli files ls "Uploads [UL]" -L
step "openkdx files ls: big.bin is listed with its full size."
cli files get "Uploads [UL]/big.bin" --to "$RUN/down.bin"
cmp "$RUN/big.bin" "$RUN/down.bin" && step "openkdx files get: the download matches the original byte for byte."
capture_end
scenario_end
