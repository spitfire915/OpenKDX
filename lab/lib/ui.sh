#!/usr/bin/env bash
# UI automation helpers for lab scenarios. Source after kdx-lab's namespace
# and X display are up. Everything goes through `kdx-lab x run`.
#
#   ui_win NAME             → window id whose title is exactly NAME (last match)
#   ui_wait NAME [SECS]     → wait until a window named NAME exists
#   ui_click WIN X Y        → left-click at X,Y relative to WIN
#   ui_rclick WIN X Y       → right-click
#   ui_type TEXT            → type text into the focused widget
#   ui_key KEYS...          → press keys (xdotool key names)
#   ui_focus WIN            → raise and focus WIN
#   ui_shot NAME            → screenshot to $CAPTURES/NAME.png
#   ui_close WIN            → close WIN via its left-hand X button

LAB="${LAB:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/kdx-lab}"
X() { "$LAB" x run "$@"; }

ui_win() {
  # ui_win NAME [CLASS] → last window titled exactly NAME, optionally with WM_CLASS CLASS
  local ids id out=""
  ids=$(X xdotool search --name "^$1\$" 2>/dev/null || true)
  for id in $ids; do
    if [[ -n "${2:-}" ]]; then
      X xprop -id "$id" WM_CLASS 2>/dev/null | grep -q "\"$2\"" || continue
    fi
    out=$id
  done
  [[ -n "$out" ]] && echo "$out"
}

ui_wait() {
  # ui_wait NAME [SECS] [CLASS]
  local name=$1 secs=${2:-20} i
  for ((i = 0; i < secs * 5; i++)); do
    [[ -n "$(ui_win "$name" "${3:-}")" ]] && return 0
    sleep 0.2
  done
  echo "ui_wait: no window named '$name' after ${secs}s" >&2
  return 1
}

ui_focus() { X xdotool windowactivate --sync "$1" 2>/dev/null || X xdotool windowraise "$1"; X xdotool windowfocus "$1" 2>/dev/null || true; }
ui_click() { X xdotool mousemove --window "$1" "$2" "$3" click 1; sleep 0.4; }
ui_rclick() { X xdotool mousemove --window "$1" "$2" "$3" click 3; sleep 0.4; }
ui_type() { X xdotool type --delay 30 -- "$1"; sleep 0.2; }
ui_key() { X xdotool key --delay 80 "$@"; sleep 0.3; }
ui_shot() { "$LAB" x shot "$1" >/dev/null; }
ui_close() { ui_click "$1" 8 8; }

# ui_geom WIN → "W H" of the window
ui_geom() { X xdotool getwindowgeometry --shell "$1" | awk -F= '/^WIDTH/{w=$2}/^HEIGHT/{h=$2}END{print w, h}'; }

# ui_win_sized NAME WxH [CLASS] → id of the window titled NAME with that exact size
ui_win_sized() {
  local name=$1 size=$2 id g
  for id in $(X xdotool search --name "^$name\$" 2>/dev/null || true); do
    if [[ -n "${3:-}" ]]; then X xprop -id "$id" WM_CLASS 2>/dev/null | grep -q "\"$3\"" || continue; fi
    g=$(ui_geom "$id")
    [[ "${g/ /x}" == "$size" ]] && { echo "$id"; return 0; }
  done
  return 1
}

# ui_wait_sized NAME WxH [SECS] [CLASS]
ui_wait_sized() {
  local i
  for ((i = 0; i < ${3:-20} * 5; i++)); do
    ui_win_sized "$1" "$2" "${4:-}" && return 0
    sleep 0.2
  done
  return 1
}

# Accept the Haxial licence and set the user name on a fresh client.
# CLASS is kdxclient.exe (Windows under Wine) or KDX (Linux beta 5).
ui_first_run_client() {
  local name=${1:-labuser} class=${2:-kdxclient.exe} w
  if ui_wait "Software License Agreement" 30 "$class"; then
    w=$(ui_win "Software License Agreement" "$class")
    ui_click "$w" 328 398   # "I Agree"
  fi
  sleep 2
  # "Please enter your name:" dialog is titled KDX, 282x144.
  if w=$(ui_wait_sized KDX 282x144 10 "$class"); then
    ui_focus "$w"; ui_click "$w" 160 75; ui_type "$name"; ui_click "$w" 224 110   # OK
  fi
  sleep 1
  ui_dismiss_dcc_error "$class"
}

# A second client in the same namespace cannot bind DCC port 10600 and shows
# "Error while trying to listen for incoming DCC". Harmless; press OK.
ui_dismiss_dcc_error() {
  local w
  sleep 1
  if w=$(ui_win Error "${1:-}"); then
    ui_focus "$w"; ui_click "$w" 346 133   # OK
  fi
}
ui_first_run_winclient() { ui_first_run_client "${1:-labuser}" kdxclient.exe; }
