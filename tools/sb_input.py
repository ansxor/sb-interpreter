"""
SmileBASIC input tool: window management + touch/button input via Luma3DS Input Redirection.

Window layout (Citra/Azahar emulator, resize target: 400x480 content):
  ┌──────────────────────┐ ← title bar (~28px macOS)
  │   Top Screen         │ 400x240
  │   (400x240)          │
  ├──────────────────────┤ ← y = title_bar + 240
  │  │ Bottom Screen │   │ 320x240 centered in 400px
  │  │   (320x240)   │   │ side margins: 40px each
  │  │               │   │
  └──────────────────────┘
  Total content area: 400x480
  Total window frame: 400 x (480 + title_bar_height)

Touch coordinate mapping (3DS touch → UDP):
  3DS touch hardware uses 0-4095 range on each axis.
  Bottom screen is 320x240 pixels.

  For computer_use / window-relative coordinates → 3DS touch:
    x_3ds = (window_x - H_MARGIN) / BOTTOM_W * TOUCH_MAX
    y_3ds = (window_y - title_bar - TOP_H) / BOTTOM_H * TOUCH_MAX
    clamped to [0, TOUCH_MAX - 1]

  Where:
    H_MARGIN = (400 - 320) / 2 = 40
    TOP_H = 240
    BOTTOM_W = 320
    BOTTOM_H = 240
    TOUCH_MAX = 4096
    title_bar = 28 (macOS default; adjustable)
"""

import subprocess
import sys
import math
from pathlib import Path
from typing import Optional, Tuple

# ── window geometry constants ──────────────────────────────────────────
TOP_W, TOP_H = 400, 240
BOTTOM_W, BOTTOM_H = 320, 240
WINDOW_W = TOP_W  # 400
WINDOW_H = TOP_H + BOTTOM_H  # 480
H_MARGIN = (TOP_W - BOTTOM_W) // 2  # 40 — centers bottom screen
# Azahar 2125.1.2 with window frame 400x539 → content is 400x480
# Title bar + chrome = 539 - 480 = 59px
# For other emulators/versions, measure: (window frame height) - (visible screen content height)
MACOS_TITLE_BAR_DEFAULT = 59

# ── 3DS touch constants ────────────────────────────────────────────────
TOUCH_MAX = 4096  # 3DS touch controller resolution (0-4095)
TOUCH_MAX_F = float(TOUCH_MAX)


# ═══════════════════════════════════════════════════════════════════════
# Coordinate mapping
# ═══════════════════════════════════════════════════════════════════════

def window_to_3ds(
    wx: int,
    wy: int,
    title_bar_height: int = MACOS_TITLE_BAR_DEFAULT,
) -> Tuple[int, int]:
    """Convert window-relative pixel coords to 3DS touch coords (0-4095 each).

    Only valid for clicks inside the bottom-screen area:
      wx in [H_MARGIN, H_MARGIN + BOTTOM_W)
      wy in [title_bar_height + TOP_H, title_bar_height + TOP_H + BOTTOM_H)
    """
    tx = (wx - H_MARGIN) / BOTTOM_W * TOUCH_MAX_F
    ty = (wy - title_bar_height - TOP_H) / BOTTOM_H * TOUCH_MAX_F
    return (
        max(0, min(TOUCH_MAX - 1, int(tx))),
        max(0, min(TOUCH_MAX - 1, int(ty))),
    )


def is_bottom_screen(
    wx: int,
    wy: int,
    title_bar_height: int = MACOS_TITLE_BAR_DEFAULT,
) -> bool:
    """Check if window coords fall within the bottom (touch) screen area."""
    return (
        H_MARGIN <= wx < H_MARGIN + BOTTOM_W
        and (title_bar_height + TOP_H) <= wy < (title_bar_height + WINDOW_H)
    )


def window_to_3ds_or_none(
    wx: int,
    wy: int,
    title_bar_height: int = MACOS_TITLE_BAR_DEFAULT,
) -> Optional[Tuple[int, int]]:
    """Convert window coords to 3DS touch coords, returning None if outside bottom screen."""
    if is_bottom_screen(wx, wy, title_bar_height):
        return window_to_3ds(wx, wy, title_bar_height)
    return None


def print_coord_map(title_bar_height: int = MACOS_TITLE_BAR_DEFAULT):
    """Print the coordinate mapping for reference (used in skills/docs)."""
    print(f"Window layout (content area): {WINDOW_W}x{WINDOW_H}")
    print(f"  Top screen:    0,0 → {TOP_W}x{TOP_H}")
    print(f"  Bottom screen: {H_MARGIN},{TOP_H} → {BOTTOM_W}x{BOTTOM_H}")
    print(f"  Title bar:     {title_bar_height}px (add to y for window coords)")
    print()
    print("Content → 3DS touch mapping:")
    print(f"  x_3ds = (cx - {H_MARGIN}) / {BOTTOM_W} * {TOUCH_MAX}")
    print(f"  y_3ds = (cy - {TOP_H}) / {BOTTOM_H} * {TOUCH_MAX}")
    print()
    print("Window → 3DS touch mapping:")
    print(f"  x_3ds = (wx - {H_MARGIN}) / {BOTTOM_W} * {TOUCH_MAX}")
    print(f"  y_3ds = (wy - {title_bar_height} - {TOP_H}) / {BOTTOM_H} * {TOUCH_MAX}")
    print()
    print("Corners (content coords → 3DS touch coords):")
    # Content coordinates: y is 0..479 (no title bar)
    content_corners = [
        ("top-left",     H_MARGIN, TOP_H),
        ("top-right",    H_MARGIN + BOTTOM_W - 1, TOP_H),
        ("bottom-left",  H_MARGIN, TOP_H + BOTTOM_H - 1),
        ("bottom-right", H_MARGIN + BOTTOM_W - 1, TOP_H + BOTTOM_H - 1),
        ("center",       H_MARGIN + BOTTOM_W // 2, TOP_H + BOTTOM_H // 2),
    ]
    for label, cx, cy in content_corners:
        # Content-to-3DS: no title bar subtraction
        tx = int((cx - H_MARGIN) / BOTTOM_W * TOUCH_MAX_F)
        ty = int((cy - TOP_H) / BOTTOM_H * TOUCH_MAX_F)
        tx = max(0, min(TOUCH_MAX - 1, tx))
        ty = max(0, min(TOUCH_MAX - 1, ty))
        print(f"  {label:12s}  ({cx:3d},{cy:3d}) → ({tx:4d},{ty:4d})")
    print()
    print("Same corners in window coords:")
    for label, cx, cy in content_corners:
        wx = cx
        wy = cy + title_bar_height
        tx, ty = window_to_3ds(wx, wy, title_bar_height)
        print(f"  {label:12s}  ({wx:3d},{wy:3d}) → ({tx:4d},{ty:4d})")


# ═══════════════════════════════════════════════════════════════════════
# Window management (macOS)
# ═══════════════════════════════════════════════════════════════════════

CITRA_PROCESS_NAMES = ["citra", "citra-qt", "azahar", "azahar-qt"]


def _find_emulator_process() -> Optional[str]:
    """Find which emulator process is running. Returns the process name or None."""
    import subprocess
    for name in CITRA_PROCESS_NAMES:
        try:
            result = subprocess.run(
                ["pgrep", "-x", name],
                capture_output=True, text=True, timeout=5,
            )
            if result.returncode == 0 and result.stdout.strip():
                return name
        except Exception:
            pass
    return None


def resize_emulator(
    x: int = 100,
    y: int = 100,
    width: int = WINDOW_W,
    height: int = WINDOW_H,
    title_bar_height: int = MACOS_TITLE_BAR_DEFAULT,
    process_name: Optional[str] = None,
) -> bool:
    """Resize the Citra/Azahar window via AppleScript.

    Args:
        x, y: Top-left screen position of the window.
        width, height: Content area size (default: 400x480).
        title_bar_height: Extra height added for the macOS title bar.
        process_name: Emulator process name (auto-detected if None).

    Returns:
        True on success.
    """
    if process_name is None:
        process_name = _find_emulator_process()
    if process_name is None:
        print("ERROR: No Citra/Azahar process found. Tried:", CITRA_PROCESS_NAMES)
        return False

    total_height = height + title_bar_height
    script = f'''
tell application "System Events"
    tell process "{process_name}"
        set position of window 1 to {{{x}, {y}}}
        set size of window 1 to {{{width}, {total_height}}}
    end tell
end tell
'''
    try:
        result = subprocess.run(
            ["osascript", "-e", script],
            capture_output=True, text=True, timeout=10,
        )
        if result.returncode == 0:
            print(
                f"Resized {process_name} window to {width}x{total_height} "
                f"(content: {width}x{height}) at ({x},{y})"
            )
            return True
        else:
            print(f"AppleScript error: {result.stderr.strip()}")
            return False
    except Exception as e:
        print(f"Failed to resize window: {e}")
        return False


def get_window_geometry(process_name: Optional[str] = None) -> Optional[dict]:
    """Get current emulator window position and size via AppleScript.

    Returns dict with keys: x, y, width, height (window frame, not content).
    """
    if process_name is None:
        process_name = _find_emulator_process()
    if process_name is None:
        return None

    script = f'''
tell application "System Events"
    tell window 1 of process "{process_name}"
        set wPos to position
        set wSize to size
        return (item 1 of wPos as string) & "," & ¬
               (item 2 of wPos as string) & "," & ¬
               (item 1 of wSize as string) & "," & ¬
               (item 2 of wSize as string)
    end tell
end tell
'''
    try:
        result = subprocess.run(
            ["osascript", "-e", script],
            capture_output=True, text=True, timeout=10,
        )
        if result.returncode == 0:
            parts = result.stdout.strip().split(",")
            if len(parts) == 4:
                return {
                    "x": int(parts[0]),
                    "y": int(parts[1]),
                    "width": int(parts[2]),
                    "height": int(parts[3]),
                }
    except Exception:
        pass
    return None


# ═══════════════════════════════════════════════════════════════════════
# Input redirection (wraps inputredirection.py)
# ═══════════════════════════════════════════════════════════════════════

def _get_inputredirection():
    """Import inputredirection module (from tools/inputredirection.py)."""
    tools_dir = Path(__file__).resolve().parent
    if str(tools_dir) not in sys.path:
        sys.path.insert(0, str(tools_dir))
    import inputredirection
    return inputredirection


# Singleton connection (lazy)
_connection = None


def get_connection(host: str = "10.0.0.58"):
    """Get or create the input redirection UDP connection."""
    global _connection
    if _connection is None:
        ir = _get_inputredirection()
        _connection = ir.Connection(host)
    return _connection


def touch(x_3ds: int, y_3ds: int, host: str = "10.0.0.58"):
    """Send a touch at 3DS coordinates (0-4095 each)."""
    conn = get_connection(host)
    conn.send_touch(x_3ds, y_3ds)


def clear_touch(host: str = "10.0.0.58"):
    """Release the touch screen."""
    conn = get_connection(host)
    conn.clear_touch()


def touch_at_window(
    wx: int,
    wy: int,
    host: str = "10.0.0.58",
    title_bar_height: int = MACOS_TITLE_BAR_DEFAULT,
):
    """Send a touch at window-relative pixel coordinates.

    Automatically maps to 3DS touch coords (only if within bottom screen area).
    Returns the 3DS coords used, or None if coords are outside bottom screen.
    """
    result = window_to_3ds_or_none(wx, wy, title_bar_height)
    if result is None:
        print(f"({wx},{wy}) is outside bottom screen area — touch not sent")
        return None
    tx, ty = result
    touch(tx, ty, host)
    return (tx, ty)


def button(button_name: str, host: str = "10.0.0.58"):
    """Send a button press (one-shot). Valid names: A, B, X, Y, L, R, START, SELECT, UP, DOWN, LEFT, RIGHT."""
    ir = _get_inputredirection()
    button_map = {
        "A": ir.HidButtonCodes.A,
        "B": ir.HidButtonCodes.B,
        "X": ir.HidButtonCodes.X,
        "Y": ir.HidButtonCodes.Y,
        "L": ir.HidButtonCodes.L,
        "R": ir.HidButtonCodes.R,
        "START": ir.HidButtonCodes.START,
        "SELECT": ir.HidButtonCodes.SELECT,
        "UP": ir.HidButtonCodes.UP,
        "DOWN": ir.HidButtonCodes.DOWN,
        "LEFT": ir.HidButtonCodes.LEFT,
        "RIGHT": ir.HidButtonCodes.RIGHT,
    }
    code = button_map.get(button_name.upper())
    if code is None:
        raise ValueError(f"Unknown button: {button_name}. Valid: {list(button_map.keys())}")
    conn = get_connection(host)
    conn.send_button_oneshot(code)


# ═══════════════════════════════════════════════════════════════════════
# CLI
# ═══════════════════════════════════════════════════════════════════════

if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="SmileBASIC input tool")
    sub = parser.add_subparsers(dest="cmd")

    # sb_input.py resize
    p_resize = sub.add_parser("resize", help="Resize emulator window to 400x480")
    p_resize.add_argument("--x", type=int, default=100)
    p_resize.add_argument("--y", type=int, default=100)
    p_resize.add_argument("--title-bar", type=int, default=MACOS_TITLE_BAR_DEFAULT)

    # sb_input.py coords
    p_coords = sub.add_parser("coords", help="Print coordinate mapping reference")
    p_coords.add_argument("--title-bar", type=int, default=MACOS_TITLE_BAR_DEFAULT)

    # sb_input.py touch <x_3ds> <y_3ds>
    p_touch = sub.add_parser("touch", help="Send touch at 3DS coords (0-4095)")
    p_touch.add_argument("x", type=int)
    p_touch.add_argument("y", type=int)
    p_touch.add_argument("--host", default="10.0.0.58")

    # sb_input.py touch-at <wx> <wy>
    p_touch_at = sub.add_parser("touch-at", help="Send touch at window coords")
    p_touch_at.add_argument("wx", type=int)
    p_touch_at.add_argument("wy", type=int)
    p_touch_at.add_argument("--host", default="10.0.0.58")
    p_touch_at.add_argument("--title-bar", type=int, default=MACOS_TITLE_BAR_DEFAULT)

    # sb_input.py clear
    p_clear = sub.add_parser("clear", help="Release touch")
    p_clear.add_argument("--host", default="10.0.0.58")

    # sb_input.py button <name>
    p_button = sub.add_parser("button", help="Press a button (A, B, X, Y, etc.)")
    p_button.add_argument("name")
    p_button.add_argument("--host", default="10.0.0.58")

    # sb_input.py geo
    p_geo = sub.add_parser("geo", help="Print current emulator window geometry")

    args = parser.parse_args()

    if args.cmd == "resize":
        resize_emulator(x=args.x, y=args.y, title_bar_height=args.title_bar)
    elif args.cmd == "coords":
        print_coord_map(title_bar_height=args.title_bar)
    elif args.cmd == "touch":
        touch(args.x, args.y, host=args.host)
    elif args.cmd == "touch-at":
        result = touch_at_window(args.wx, args.wy, host=args.host, title_bar_height=args.title_bar)
        if result:
            print(f"Touch sent at 3DS coords: {result}")
    elif args.cmd == "clear":
        clear_touch(host=args.host)
    elif args.cmd == "button":
        button(args.name, host=args.host)
    elif args.cmd == "geo":
        info = get_window_geometry()
        if info:
            print(f"Window: {info['width']}x{info['height']} at ({info['x']},{info['y']})")
        else:
            print("No emulator window found")
    else:
        parser.print_help()
