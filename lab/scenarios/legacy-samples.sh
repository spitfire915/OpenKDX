#!/usr/bin/env bash
# Legacy-format samples: one controlled change per snapshot of the server's
# data directory, so that each on-disk format can be worked out by diffing.
# Not a capture scenario; output goes to $RUN/legacy/<NN-name>/ (the four
# server files after each step) and is copied into fixtures/legacy/ by hand
# once checked for Haxial prose.
#
# Steps marked (UI) go through the Windows client's Server Settings window;
# the coordinates are relative to that window (432x378) and its dialogs.
source "$(dirname "$0")/../lib/scenario.sh"
source "$(dirname "$0")/../lib/ui.sh"
OUT="$RUN/legacy"
rm -rf "$OUT"; mkdir -p "$OUT"

snap() { # snap NAME : copy the server files as they are now
  local d="$OUT/$1"; mkdir -p "$d"; sleep 1.5
  for f in Accounts.dat KDXServer.stg News.dat; do [[ -f "$RUN/server/$f" ]] && cp "$RUN/server/$f" "$d/"; done
  [[ -d "$RUN/server/History" ]] && cp -r "$RUN/server/History" "$d/"
  [[ -d "$RUN/server/News Archives" ]] && cp -r "$RUN/server/News Archives" "$d/"
  echo "snap $1: $(cd "$d" && ls -l | awk 'NR>1{printf "%s=%s ", $NF, $5}')"
}

# --- Server Settings window --------------------------------------------------
SS=""
open_server_settings() {
  SS=$(ui_win "Server Settings @ 127.0.0.1:10700" "$CLASS" || true)
  [[ -n "$SS" ]] && return
  ui_focus "$PALETTE"; ui_click "$PALETTE" 80 79; sleep 1      # Commands
  X xdotool mousemove 100 115 click 1; sleep 1.5               # 127.0.0.1:10700 ▸
  X xdotool mousemove 300 119; sleep 0.5; X xdotool mousemove 300 300; sleep 0.3
  X xdotool mousemove 300 497; sleep 0.7; X xdotool mousedown 1; sleep 0.3; X xdotool mouseup 1
  ui_wait "Server Settings @ 127.0.0.1:10700" 10 "$CLASS"
  SS=$(ui_win "Server Settings @ 127.0.0.1:10700" "$CLASS")
  for c in $(X xdotool search --name '^KDX Connect$' 2>/dev/null || true); do ui_close "$c"; done
  X xdotool windowmove "$SS" 440 20; sleep 0.5
}
press_nf() { X xdotool mousemove --window "$1" "$2" "$3"; sleep 0.2; X xdotool mousedown 1; sleep 0.2; X xdotool mouseup 1; sleep 0.6; }
press() { ui_focus "$1"; press_nf "$@"; }
# Buttons want a pause between the move and the click.
tap() { X xdotool mousemove --window "$1" "$2" "$3"; sleep 0.3; X xdotool click 1; sleep 0.8; }
ss_page() { # general|ports|greeting|allowdeny|history|trackers
  local y; case $1 in general) y=60;; ports) y=76;; greeting) y=92;; allowdeny) y=108;; history) y=124;; trackers) y=140;; esac
  press "$SS" 120 39; sleep 0.8; press_nf "$SS" 80 "$y"; sleep 1.2
}
ss_save() { press "$SS" 379 39; for _ in $(seq 1 20); do [[ -z "$(ui_win "Server Settings @ 127.0.0.1:10700" "$CLASS" || true)" ]] && break; sleep 0.3; done; sleep 1; }
ss_field() { press "$SS" "$1" "$2"; ui_type "$3"; }
ss_replace() { press "$SS" "$1" "$2"; ui_key End; for _ in $(seq 1 "${4:-8}"); do ui_key BackSpace; done; for _ in $(seq 1 "${4:-8}"); do ui_key Delete; done; ui_type "$3"; }

FORCE=1 scenario_begin legacy-samples
use_cli
start_server
snap 00-fresh
cli server set-name "Legacy Lab";                 snap 01-name
start_winclient
connect_to 127.0.0.1 "$KDX_ADMIN_LOGIN" "$KDX_ADMIN_PASSWORD"; sleep 3
# The window closes itself after Save, so every change reopens it.
ss_begin() { open_server_settings; ss_page "$1"; }
ss_begin general; ss_field 300 145 "A description";  ss_save;  snap 02-description
ss_begin general; ss_field 300 214 "lab.example";    ss_save;  snap 03-address
ss_begin general; press "$SS" 24 331; ss_replace 290 330 "20" 4; ss_save; snap 04-speed
ss_begin ports;   ss_replace 235 144 "10701" 6;      ss_save;  snap 05-altport
ss_begin greeting; press "$SS" 24 77; ss_field 300 200 "Welcome to the lab"; ss_save; snap 06-greeting
ss_begin history; press "$SS" 54 100;                ss_save;  snap 07-history-login
ss_begin history; press "$SS" 54 120;                ss_save;  snap 08-history-disc
ss_begin history; ss_replace 305 334 "512" 6;        ss_save;  snap 09-history-size
ss_begin allowdeny
tap "$SS" 80 81; ui_wait "Edit A/D Address" 5 "$CLASS"; D=$(ui_win "Edit A/D Address" "$CLASS" || true)
press "$D" 230 44; ui_type "Bad host"; press "$D" 230 74; ui_type "10.9.8.7"; tap "$D" 302 275; sleep 1
ss_save;                                                        snap 10-deny
ss_begin allowdeny
tap "$SS" 80 81; ui_wait "Edit A/D Address" 5 "$CLASS"; D=$(ui_win "Edit A/D Address" "$CLASS" || true)
press "$D" 230 44; ui_type "Good host"; press "$D" 230 74; ui_type "10.1.1.1"; press "$D" 124 100; press "$D" 24 134; tap "$D" 302 275; sleep 1
ss_save;                                                        snap 11-allow-expiry
ss_begin trackers
tap "$SS" 85 81; ui_wait "Edit Tracker" 5 "$CLASS"; T=$(ui_win "Edit Tracker" "$CLASS" || true)
press "$T" 230 44; ui_type "Lab tracker"; press "$T" 230 136; ui_type "127.0.0.1"; tap "$T" 369 341; sleep 1
ss_save;                                                        snap 12-tracker
ss_begin trackers; press "$SS" 184 81;               ss_save;  snap 13-register
ss_begin ports;   ss_replace 235 174 "10702" 6;      ss_save;  snap 14-ftport
ss_begin trackers
tap "$SS" 85 81; ui_wait "Edit Tracker" 5 "$CLASS"; T=$(ui_win "Edit Tracker" "$CLASS" || true)
press "$T" 230 44; ui_type "Second"; press "$T" 230 90; ui_type "tracker comment"; press "$T" 230 136; ui_type "10.0.0.2"
press "$T" 230 166; ui_type "tlogin"; press "$T" 230 196; sleep 0.3; ui_type "tpass"; press "$T" 230 266; ui_type "grp"; ui_shot legacy-samples-tracker-dialog; tap "$T" 369 341; sleep 1
ss_save;                                                        snap 15-tracker-full
ss_begin allowdeny
tap "$SS" 80 81; ui_wait "Edit A/D Address" 5 "$CLASS"; D=$(ui_win "Edit A/D Address" "$CLASS" || true)
press "$D" 230 44; ui_type "Noted"; press "$D" 230 74; ui_type "10.5.5.5"; press "$D" 180 220; ui_type "a comment"; tap "$D" 302 275; sleep 1
ss_save;                                                        snap 16-deny-comment
ss_begin general; ui_shot legacy-samples-settings; ui_close "$SS"
echo "settings snapshots done"

# --- Accounts.dat: classes, accounts, folder access, through the CLI ---------
cli accounts create legacy1 --new-password p1 --name "Name X" --class 1 >/dev/null; snap 20-account-created
cli accounts set legacy1 --name "Name Y" >/dev/null;                               snap 21-account-renamed
cli accounts set legacy1 --new-password p2 >/dev/null;                            snap 22-account-password
cli classes set 3 --name "Custom Renamed" >/dev/null;                             snap 23-class-renamed
cli folder-access create Pics --icon pictures >/dev/null;                         snap 24-folder-access
cli accounts create legacy2 --new-password p2 --class 2 >/dev/null;               snap 25-second-account
cli accounts delete legacy2 >/dev/null;                                           snap 26-account-deleted
echo "accounts snapshots done"

# --- News.dat and News Archives/ ---------------------------------------------
# The stock welcome post is Haxial's text: delete it first so no snapshot
# after 30 carries it (30 itself is kept out of the repository).
cli news groups >/dev/null;                                                       snap 30-news-stock
first=$(cli --json news list "General News" | python3 -c 'import json,sys; rows=[json.loads(l) for l in sys.stdin if l.startswith("{")]; print(min(r["message_id"] for r in rows if "message_id" in r))' 2>/dev/null || echo 1)
cli news delete "General News" "$first" >/dev/null;                               snap 31-news-empty
cli news post "General News" --subject "First" --body "Hello from the lab." >/dev/null; snap 32-news-one-post
cli news create-group "Second Group" --description "Another group" --size-kib 8 >/dev/null; snap 33-news-two-groups
cli news post "Second Group" --subject "In second" --body "Line one.
Line two." >/dev/null;                                                            snap 34-news-post-in-second
cli news delete "General News" "$(( first + 1 ))" >/dev/null;                     snap 35-news-deleted
cli news set-group-size "Second Group" 4 >/dev/null;                              snap 36-news-size-4
python3 -c 'print("x" * 1500)' > "$RUN/big.txt"
for i in 1 2 3 4; do cli news post "Second Group" --subject "Big $i" --body-file "$RUN/big.txt" >/dev/null; done
sleep 2;                                                                          snap 37-news-archived
echo "news snapshots done"
