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

# Haxial popup menus are Mac-style: press on the control, drag to the item,
# release, then one more click commits. ui_popup WIN X Y ROW: open the popup
# at X,Y (relative to WIN) and choose the ROWth item (1-based, 16 px rows).
ui_popup() {
  local win=$1 x=$2 y=$3 row=$4 wx wy ax ay i
  read -r wx wy <<<"$(X xdotool getwindowgeometry --shell "$win" | awk -F= '/^X=/{x=$2}/^Y=/{y=$2}END{print x, y}')"
  ax=$((wx + x)); ay=$((wy + y))
  X xdotool mousemove "$ax" "$ay" mousedown 1; sleep 0.6
  for ((i = 1; i <= row; i++)); do X xdotool mousemove $((ax - 20)) $((ay + 5 + i * 16)); sleep 0.12; done
  X xdotool mouseup 1; sleep 0.4
  X xdotool click 1; sleep 1
}

# ui_menu_path: open the palette Commands menu, hover the first (server) item,
# and move down ROW rows in its submenu; click.
ui_commands_server_item() {
  local palette=$1 row=$2 px py i
  read -r px py <<<"$(X xdotool getwindowgeometry --shell "$palette" | awk -F= '/^X=/{x=$2}/^Y=/{y=$2}END{print x, y}')"
  ui_focus "$palette"; ui_click "$palette" 80 79; sleep 1
  X xdotool mousemove $((px + 100)) $((py + 105)) click 1; sleep 1
  X xdotool mousemove $((px + 290)) $((py + 108)); sleep 0.4
  for ((i = 1; i <= row; i++)); do X xdotool mousemove $((px + 290)) $((py + 108 + i * 19)); sleep 0.05; done
  sleep 0.3; X xdotool click 1; sleep 1.5
}
