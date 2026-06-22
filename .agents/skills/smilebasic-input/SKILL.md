---
name: smilebasic-input
description: |
  SmileBASIC 3DS touch/button input via Luma3DS Input Redirection plus
  Citra/Azahar emulator window resize to 400x480. Coordinate mapping for
  both content coords and computer_use window coords.
version: 1.0.0
platforms: [macos]
metadata:
  hermes:
    tags: [smilebasic, 3ds, citra, azahar, input, touch, emulator]
    category: software-development
    related_skills: [macos-computer-use]
---

# SmileBASIC Input: Window Management + Touch/Button Input

Send touch and button inputs to SmileBASIC 3.6.0 running in Citra/Azahar
emulator via Luma3DS Input Redirection (UDP port 4950).

## When to load

Load this skill when:
- Resizing the emulator window for predictable screen coordinates
- Sending touch inputs to the 3DS bottom screen
- Sending button presses (A, B, D-pad, etc.)
- Using `computer_use` to click on the emulator window
- Setting up or debugging the input pipeline for the SB interpreter harness

## Window geometry

The target window layout (Azahar 2125.1.2, Default screen layout):

```
Window frame: 400x539
  ┌──────────────────────┐ ← macOS title bar (59px in Azahar)
  │   Top Screen         │ 400x240 (3DS upper LCD)
  │   (400x240)          │ content y: 0-239
  ├──────────────────────┤ ← content y=240
  │  │ Bottom Screen │   │ 320x240 centered in 400px
  │  │   (320x240)   │   │ content y: 240-479
  │  │  TOUCH HERE   │   │ margins: 40px each side
  └──────────────────────┘
```

Key numbers:
- Content area: 400x480 (top 240 + bottom 240)
- Bottom screen: 320x240, horizontally centered -> 40px side margins
- Title bar offset: 59px (measured: window frame 539 - content 480)
- Window frame target: 400x539

**IMPORTANT**: The title bar height varies by emulator version and macOS
version. Always verify with `python3 tools/sb_input.py geo` and adjust
`--title-bar` accordingly. For Azahar 2125.1.2 on macOS 26.2: 59px.

## Resizing the window

```bash
# Resize to 400x480 content (400x539 frame with 59px title bar)
python3 tools/sb_input.py resize

# Check current geometry
python3 tools/sb_input.py geo

# Custom position
python3 tools/sb_input.py resize --x 11 --y 44

# Custom title bar (if different emulator version)
python3 tools/sb_input.py resize --title-bar 28
```

The resize uses AppleScript (`osascript`) to set window frame position and
size. It auto-detects the emulator process (tries: `azahar`, `azahar-qt`,
`citra`, `citra-qt`).

## Coordinate mapping

### Content coords -> 3DS touch (0-4095)

Used when working with content-relative coordinates (e.g., from
framebuffer.py):

```
x_3ds = (cx - 40) / 320 * 4096
y_3ds = (cy - 240) / 240 * 4096
```

### Window coords -> 3DS touch (0-4095)

Used with `computer_use` click coordinates (window-relative):

```
x_3ds = (wx - 40) / 320 * 4096
y_3ds = (wy - 59 - 240) / 240 * 4096
```

### Reference corners

| Position     | Content    | Window      | 3DS Touch   |
|-------------|------------|-------------|-------------|
| Top-left    | (40, 240)  | (40, 299)   | (0, 0)      |
| Top-right   | (359, 240) | (359, 299)  | (4083, 0)   |
| Bottom-left | (40, 479)  | (40, 538)   | (0, 4078)   |
| Bottom-right| (359, 479) | (359, 538)  | (4083, 4078)|
| Center      | (200, 360) | (200, 419)  | (2048, 2048)|

Print full reference: `python3 tools/sb_input.py coords`

## Sending touch inputs

The input pipeline uses Luma3DS Input Redirection over UDP (port 4950).
The 3DS must be running Luma3DS with Input Redirection enabled (Rosalina
menu -> Miscellaneous options -> Start Input Redirection).

### Direct 3DS coords (0-4095 each)

```bash
# Touch center of bottom screen
python3 tools/sb_input.py touch 2048 2048

# Touch top-left
python3 tools/sb_input.py touch 0 0

# Release touch
python3 tools/sb_input.py clear
```

### Window coords (auto-mapped)

```bash
# Click at window position (200, 419) -> auto-maps to 3DS (2048, 2048)
python3 tools/sb_input.py touch-at 200 419 --title-bar 59
```

### From Python

```python
from tools.sb_input import touch, clear_touch, button, window_to_3ds

# Send touch at 3DS coordinates
touch(2048, 2048)       # center of bottom screen
clear_touch()           # release

# Map window coords to 3DS coords
if is_bottom_screen(wx, wy, title_bar_height=59):
    tx, ty = window_to_3ds(wx, wy, title_bar_height=59)
    touch(tx, ty)
```

### Host configuration

Default host is `10.0.0.58` (the real 3DS IP from the original
inputredirection.py). For emulator, the Luma3DS network depends on
Citra/Azahar network config. Override with `--host`:

```bash
python3 tools/sb_input.py touch 2048 2048 --host 192.168.1.100
```

## Sending button presses

```bash
python3 tools/sb_input.py button A
python3 tools/sb_input.py button START
python3 tools/sb_input.py button UP
```

Valid buttons: A, B, X, Y, L, R, START, SELECT, UP, DOWN, LEFT, RIGHT

## Using computer_use to click

When using `computer_use` to interact with the emulator window:

1. Resize the window first: `python3 tools/sb_input.py resize`
2. Capture with app targeting: `computer_use(action='capture', app='azahar', mode='som')`
3. Click by element index (preferred) or by pixel coordinate
4. Map coordinates: Use `window_to_3ds(wx, wy, title_bar_height=59)` to convert
5. Verify: Re-capture after actions

The bottom screen touch area in window coords:
- x: 40 to 359 (320px wide)
- y: 299 to 538 (240px tall, below 59px title bar + 240px top screen)

Anything outside this range is NOT the bottom screen (either top screen,
margins, or window chrome).

## Files

- `tools/sb_input.py` — Main tool: window resize, coordinate mapping, touch/button CLI
- `tools/inputredirection.py` — Luma3DS Input Redirection client (from ilovecherries/sb-inputredirection)
- `tools/citra.py` — Citra/Azahar RPC client (memory read/write, separate from touch input)
- `harness/oracle/framebuffer.py` — Screen dimensions constants (TOP_W/H, BOTTOM_W/H)

## Pitfalls

1. **Title bar height varies**: Always verify with `sb_input.py geo` before
   mapping. Azahar 2125.1.2 = 59px. Other versions may differ.
2. **No route to host**: The 3DS/emulator must have Input Redirection
   running. Without it, `touch`/`button` commands fail with socket errors.
3. **inputredirection.py import side effect**: The original script created a
   global connection at import time. Fixed — connection only created when
   running as `__main__` or explicitly via `get_connection()`.
4. **Screen layout must be "Default"**: Azahar View -> Screen Layout ->
   Default. Other layouts (Side by Side, Large Screen, etc.) have different
   geometry.
5. **3DS touch range is 0-4095**: Window pixels don't map 1:1. Always use
   the mapping functions, not raw division.
