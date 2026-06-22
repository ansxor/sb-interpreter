#!/usr/bin/env python3
"""Window control + touch for SmileBASIC in Azahar (macOS), via cliclick.

Verified primitives: raise the window (open -a Azahar), read its bounds (osascript),
tap a window-relative point as a 3DS touch (cliclick at screen point), and screenshot.

Coordinate model: a point (wx,wy) inside the window maps to screen (bx+wx, by+wy), where
(bx,by) is the window's top-left in points. cliclick uses screen POINTS. Confirmed:
window (328,275) hits the on-screen `RUN` tab.
"""
import json
import subprocess
import sys
import time
from pathlib import Path

APP = "Azahar"
PROC = "azahar"
POS = (60, 80)      # fixed window top-left (points)
SIZE = (400, 539)   # fixed window size — keeps key coords stable
KEYMAP = Path(__file__).with_name("keymap.json")


def _osa(script):
    return subprocess.run(["osascript", "-e", script], capture_output=True, text=True).stdout.strip()


def is_running():
    """True if an Azahar process is up."""
    return (subprocess.run(["pgrep", "-x", PROC], capture_output=True).returncode == 0
            or subprocess.run(["pgrep", "-if", "Azahar.app"], capture_output=True).returncode == 0)


def raise_window():
    """Bring Azahar to the front and pin its geometry. Launches it if not running (cold
    start: wait for SmileBASIC to boot). Returns (bx,by,w,h)."""
    cold = not is_running()
    subprocess.run(["open", "-a", APP])
    time.sleep(12.0 if cold else 0.8)  # cold start needs ~boot time before SB is usable
    _osa(f'tell application "System Events" to tell (first process whose name contains "{PROC}") '
         f'to set position of window 1 to {{{POS[0]}, {POS[1]}}}')
    _osa(f'tell application "System Events" to tell (first process whose name contains "{PROC}") '
         f'to set size of window 1 to {{{SIZE[0]}, {SIZE[1]}}}')
    time.sleep(0.3)
    return bounds()


def bounds():
    out = _osa(f'tell application "System Events" to tell (first process whose name contains "{PROC}") '
              f'to get {{position, size}} of window 1')
    nums = [int(n) for n in out.replace(",", " ").split()]
    return tuple(nums) if len(nums) == 4 else (*POS, *SIZE)


def tap(wx, wy, hold=0.05):
    """Tap a window-relative point (wx,wy) as a 3DS touch."""
    bx, by, _, _ = bounds()
    sx, sy = bx + int(wx), by + int(wy)
    # cliclick: down, brief hold, up (a plain click also works; this is gentler for touch).
    subprocess.run(["cliclick", f"dd:{sx},{sy}"])
    time.sleep(hold)
    subprocess.run(["cliclick", f"du:{sx},{sy}"])
    return sx, sy


def shot(path="/tmp/sb.png"):
    subprocess.run(["open", "-a", APP])
    time.sleep(0.5)
    bx, by, w, h = bounds()
    subprocess.run(["screencapture", "-x", "-o", f"-R{bx},{by},{w},{h}", path])
    return path


def load_keymap():
    return json.loads(KEYMAP.read_text()) if KEYMAP.exists() else {}


def type_str(s):
    """Type a string by tapping calibrated keys. Raises if a key isn't calibrated."""
    km = load_keymap()
    for ch in s:
        key = ch.upper()
        if key not in km:
            raise KeyError(f"key {ch!r} not in keymap.json (calibrate it first)")
        wx, wy = km[key]
        tap(wx, wy)
        time.sleep(0.08)


def press(name):
    """Tap a named key/tab (ENTER, SHIFT, BACKSPACE, RUN, DIRECT, EDIT, SPACE)."""
    km = load_keymap()
    if name not in km:
        raise KeyError(f"named key {name!r} not in keymap.json")
    return tap(*km[name])


def key_combo(modifier, key):
    """Send a keyboard chord to the focused window, e.g. key_combo('ctrl','p') = Ctrl+P
    (Azahar's Capture-Screenshot shortcut). Uses cliclick: hold modifier, type key, release."""
    subprocess.run(["cliclick", f"kd:{modifier}", f"t:{key}", f"ku:{modifier}"])


def enter():
    press("ENTER")


def clear_line():
    """Clear the current DIRECT-mode line: SHIFT then BACKSPACE (per SB)."""
    press("SHIFT")
    time.sleep(0.1)
    press("BACKSPACE")
    time.sleep(0.1)


def main():
    a = sys.argv[1:]
    if not a:
        print(__doc__); return
    cmd = a[0]
    if cmd == "raise":
        print("bounds:", raise_window())
    elif cmd == "bounds":
        print(bounds())
    elif cmd == "shot":
        print("saved:", shot(a[1] if len(a) > 1 else "/tmp/sb.png"))
    elif cmd == "tap":
        print("tapped screen:", tap(int(a[1]), int(a[2])))
    elif cmd == "calibrate":
        wx, wy = int(a[1]), int(a[2])
        shot("/tmp/sb_before.png")
        tap(wx, wy); time.sleep(0.6)
        shot("/tmp/sb_after.png")
        print(f"tapped window ({wx},{wy}); compare /tmp/sb_before.png vs /tmp/sb_after.png")
    elif cmd == "type":
        type_str(a[1])
    elif cmd == "enter":
        enter()
    elif cmd == "clear":
        clear_line()
    elif cmd == "press":
        press(a[1])
    else:
        print("usage: raise | bounds | shot [path] | tap WX WY | calibrate WX WY | "
              "type STR | enter | clear | press NAME")


if __name__ == "__main__":
    main()
