---
title: BUTTON
slug: docs-sb4-button
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-button
content_id: 19518
created: 2020-05-16
scraped: 2026-06-21
---

# BUTTON

Check the status of a controller button.

---

## Check Button

Check a specific button if it has been held, repeated, pressed, or released.

### Syntax

```sb4
state% = BUTTON(controller%, button% {, mode% })
BUTTON controller%, button% {, mode%} OUT state%
```

| Input | Description |
| --- | --- |
| `controller%` | ID of the controller to check; 0 to 4.<br>Controller 0 is a combination of all connected controllers. |
| `button%` | ID of the button to check; see *Button IDs* table. |
| `mode%` | Button test mode (optional): \| Number \| Description \|<br>\| --- \| --- \|<br>\| 0 \| Button is held (default) \|<br>\| 1 \| Button was just pressed or triggered by repeat \|<br>\| 2 \| Button was just pressed \|<br>\| 3 \| Button was just released \| |

| Output | Description |
| --- | --- |
| `state%` | True if the button is in the checked state, false otherwise. |

### Examples

```sb4
'turn the background red if the A button is held down
LOOP
 VSYNC
 IF BUTTON(0,#B_A) THEN
  BACKCOLOR #C_RED
 ELSE
  BACKCOLOR #C_CLEAR
 ENDIF
ENDLOOP
```

---

## Button Bitset (SB3 Mode)

Return the state of all buttons as a bitset. This form is similar to SmileBASIC 3's `BUTTON` function.

### Syntax

```sb4
state% = BUTTON(controller%)
state% = BUTTON(controller%, -1, {, mode% })
BUTTON controller% OUT state%
BUTTON controller%, -1 {, mode%} OUT state%
```

| Input | Description |
| --- | --- |
| `controller%` | ID of the controller to check; 0 to 4.<br>Controller 0 is a combination of all connected controllers. |
| `-1` | Pass -1 as the button ID to use this mode (optional) |
| `mode%` | Button test mode (optional): \| Number \| Description \|<br>\| --- \| --- \|<br>\| 0 \| Button is held (default) \|<br>\| 1 \| Button was just pressed or triggered by repeat \|<br>\| 2 \| Button was just pressed \|<br>\| 3 \| Button was just released \| |

| Output | Description |
| --- | --- |
| `state%` | Bitset of all buttons. Corresponding bit is set if state check is true, clear otherwise. |

### Examples

This function is used to return all of the button states simultaneously, for compatibility with previous SmileBASIC versions. However, the button ID constants are *not* compatible, so they must be used with a left-shift.

```sb4
'turn the background red if the A button is held down
LOOP
 VSYNC
 IF BUTTON(0) AND 1<<#B_A THEN
  BACKCOLOR #C_RED
 ELSE
  BACKCOLOR #C_CLEAR
 ENDIF
ENDLOOP
```

---

## Button IDs

This table contains the button IDs and their constants.

| Name | Value | Description |
| --- | --- | --- |
| `#B_RUP`, `#B_X` | 0 | Top face button / X |
| `#B_RDOWN`, `#B_B` | 1 | Bottom face button / B |
| `#B_RLEFT`, `#B_Y` | 2 | Left face button / Y |
| `#B_RRIGHT`, `#B_A` | 3 | Right face button / A |
| `#B_LUP` | 4 | D-Pad up |
| `#B_LDOWN` | 5 | D-Pad down |
| `#B_LLEFT` | 6 | D-Pad left |
| `#B_LRIGHT` | 7 | D-Pad right |
| `#B_L1`, `#B_SL` | 8 | L trigger / SL trigger |
| `#B_R1`, `#B_SR` | 9 | R trigger / SR trigger |
| `#B_L2`, `#B_S1` | 10 | ZL trigger / Joy-Con side trigger |
| `#B_R2`, `#B_S2` | 11 | ZR trigger / Joy-Con side Z trigger |
| `#B_LSTICK` | 12 | Left stick click |
| `#B_RSTICK` | 13 | Right stick click |
| `#B_RANY` | 14 | Any right side button |
| `#B_LANY` | 15 | Any left side button |
| `#B_ANY` | 16 | Any button |

## See Also

- CONTROLLER
- BREPEAT
- XCTRLSTYLE
