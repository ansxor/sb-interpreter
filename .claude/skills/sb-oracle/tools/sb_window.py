#!/usr/bin/env python3
"""Window control + touch for SmileBASIC in Azahar (macOS), via cliclick.

Verified primitives: raise the window (open -a Azahar), read its bounds (osascript),
tap a window-relative point as a 3DS touch (cliclick at screen point), and screenshot.

Coordinate model: a point (wx,wy) inside the window maps to screen (bx+wx, by+wy), where
(bx,by) is the window's top-left in points. cliclick uses screen POINTS. Confirmed:
window (328,275) hits the on-screen `RUN` tab.
"""
import glob
import json
import os
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
    (Azahar's Capture-Screenshot shortcut). Uses cliclick: hold modifier, type key, release.

    NOTE: the Ctrl+P chord does NOT reliably fire Azahar's Capture-Screenshot action (the
    render widget doesn't take the chord even when the window is frontmost). For screenshot
    capture use capture_screenshot_menu() instead, which drives the Tools menu item directly.
    Kept for other chords (none currently in use)."""
    subprocess.run(["cliclick", f"kd:{modifier}", f"t:{key}", f"ku:{modifier}"])


def capture_screenshot_menu():
    """Fire Azahar's Tools -> 'Capture Screenshot' via the menu item (NOT the Ctrl+P chord,
    which is registered but does not reliably fire). Brings Azahar frontmost, clicks the menu
    item, and returns the landed PNG path (newest file in the screenshots dir). Raises
    TimeoutError if no new screenshot appears within ~6s.

    The landed PNG is 400x480 RGB (both screens stacked, top then bottom). Callers split it
    per screen + pad alpha themselves (see run_case.capture_screen)."""
    shotdir = os.path.expanduser("~/Library/Application Support/Azahar/screenshots")
    raise_window()
    time.sleep(0.3)
    pre = set(glob.glob(os.path.join(shotdir, "*.png")))
    _osa(
        'tell application "System Events" to tell process "' + PROC + '"\n'
        '  set frontmost to true\n'
        '  delay 0.2\n'
        '  click menu item "Capture Screenshot" of menu "Tools" of menu bar 1\n'
        'end tell'
    )
    for _ in range(20):
        time.sleep(0.3)
        new = set(glob.glob(os.path.join(shotdir, "*.png"))) - pre
        if new:
            return max(new, key=os.path.getmtime)
    raise TimeoutError("no new screenshot appeared (is Azahar running + SB loaded?)")


def enter():
    press("ENTER")


def clear_line():
    """Clear the current DIRECT-mode line: SHIFT then BACKSPACE (per SB)."""
    press("SHIFT")
    time.sleep(0.1)
    press("BACKSPACE")
    time.sleep(0.1)


# ── SAVE-dialog handling ─────────────────────────────────────────────────────────────────
# A SmileBASIC SAVE is a TWO-dialog sequence on the bottom screen:
#   1. "Confirm - Write file ... Do you want to proceed?"  -> tap YES (file is written)
#   2. "Information - Write file ... written successfully"  -> tap OK  (same coord as YES)
# Both dialogs COVER the keyboard with a light/cream body; the keyboard is dark. So we can
# tell a dialog is open by sampling one point on the bottom screen and checking brightness,
# then tap the YES/OK button until NO dialog remains. This makes every save self-closing —
# no speculative "clear a stale dialog" tap (that one mis-fired onto a key when none was open).

def _decode_png(path):
    """Minimal PNG decode -> (w, h, channels, raw RGBA/RGB bytes). stdlib only (zlib)."""
    import struct
    import zlib
    d = open(path, "rb").read()
    if d[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError("not a PNG")
    off, w, h, ct, idat = 8, 0, 0, 0, b""
    while off < len(d):
        ln = struct.unpack(">I", d[off:off + 4])[0]
        typ = d[off + 4:off + 8]
        data = d[off + 8:off + 8 + ln]
        if typ == b"IHDR":
            w, h, _bd, ct = struct.unpack(">IIBB", data[:10])
        elif typ == b"IDAT":
            idat += data
        elif typ == b"IEND":
            break
        off += 12 + ln
    raw = zlib.decompress(idat)
    ch = {0: 1, 2: 3, 4: 2, 6: 4}[ct]
    stride = w * ch
    out = bytearray(h * stride)
    prev = bytearray(stride)
    pos = 0
    for y in range(h):
        f = raw[pos]; pos += 1
        line = bytearray(raw[pos:pos + stride]); pos += stride
        if f == 1:
            for i in range(ch, stride):
                line[i] = (line[i] + line[i - ch]) & 255
        elif f == 2:
            for i in range(stride):
                line[i] = (line[i] + prev[i]) & 255
        elif f == 3:
            for i in range(stride):
                a = line[i - ch] if i >= ch else 0
                line[i] = (line[i] + ((a + prev[i]) >> 1)) & 255
        elif f == 4:
            for i in range(stride):
                a = line[i - ch] if i >= ch else 0
                b = prev[i]
                c = prev[i - ch] if i >= ch else 0
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                pr = a if (pa <= pb and pa <= pc) else (b if pb <= pc else c)
                line[i] = (line[i] + pr) & 255
        out[y * stride:(y + 1) * stride] = line
        prev = line
    return w, h, ch, bytes(out)


def region_brightness(wx, wy, size=18):
    """Mean RGB brightness (0-255) of a small box around window point (wx,wy). Captures only
    that region (fast, no full-window decode)."""
    bx, by, _, _ = bounds()
    sx, sy = bx + wx - size // 2, by + wy - size // 2
    p = "/tmp/_sb_px.png"
    subprocess.run(["screencapture", "-x", "-o", f"-R{sx},{sy},{size},{size}", p])
    w, h, ch, px = _decode_png(p)
    tot = n = 0
    for i in range(0, w * h * ch, ch):
        tot += px[i] + px[i + 1] + px[i + 2]
        n += 3
    return tot / n if n else 0.0


def dialog_open(sample=(180, 380), thresh=120.0):
    """True if a SAVE dialog (Confirm or Information) is covering the bottom screen. The
    dialog body is light (>=158); the keyboard at this point is dark (<=75)."""
    return region_brightness(*sample) > thresh


def confirm_dialogs(rounds=8, settle=0.9):
    """Close every open SAVE dialog by tapping the YES/OK button (one screen position serves
    both) until no dialog remains. Handles the two-step Confirm->Information sequence, and is
    a safe no-op when nothing is open. Returns True if the screen is dialog-free at the end."""
    for _ in range(rounds):
        if not dialog_open():
            return True
        press("YES")
        time.sleep(settle)
    return not dialog_open()


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
    elif cmd == "dialog":
        print(f"brightness={region_brightness(180, 380):.0f} -> "
              f"{'DIALOG OPEN' if dialog_open() else 'no dialog'}")
    elif cmd == "confirm":
        print("dialog-free:" , confirm_dialogs())
    else:
        print("usage: raise | bounds | shot [path] | tap WX WY | calibrate WX WY | "
              "type STR | enter | clear | press NAME")


if __name__ == "__main__":
    main()
