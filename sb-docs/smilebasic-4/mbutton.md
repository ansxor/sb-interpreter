---
title: MBUTTON
slug: docs-sb4-mbutton
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-mbutton
content_id: 19524
created: 2021-12-02
scraped: 2026-06-21
---

# MBUTTON

Read the status of mouse buttons. This function is very similar to [`BUTTON`](https://smilebasicsource.com/forum/thread/1335).

## Check Button

Check a specific button if it has been held, pressed, or released.

```sbfunction
MBUTTON button% {, mode% } OUT state%
```

| Input | Description |
| --- | --- |
| `button%` | ID of the button to check. See the Button IDs table. |
| `mode%` | Button test mode (optional): — Number: 0, Description: Button is held (default)<br>— Number: 1, Description: /invalid/<br>— Number: 2, Description: Button was just pressed<br>— Number: 3, Description: Button was just released |

| Output | Description |
| --- | --- |
| `state%` | True if the button is in the checked state, false otherwise. |

---

## Button Bitset

Return the state of all buttons as a bitset. Unlike `BUTTON` and the above, you can only use this to see which buttons are held.

```sbfunction
MBUTTON OUT bits%
```

| Output | Description |
| --- | --- |
| `bits%` | A bitset containing which buttons are held. See the Button IDs table. |

---

## Button IDs

This table contains the IDs assigned to each mouse button. To determine which bit to test in bitset mode, use `1 << id`.

Most mice have only the left, middle, and right buttons. If a mouse does have extra buttons, they may not be assigned to buttons 4 and 5 in the USB mouse protocol. Some mice require special driver software to assign extra buttons, which is not compatible with SmileBASIC, and SmileBASIC cannot recognize more than 5 standard mouse buttons. Design programs with this in mind, and make sure to test your mouse.

| ID | Button |
| --- | --- |
| 0 | Left Button |
| 1 | Right Button |
| 2 | Middle Button (Wheel Click) |
| 3 | Button 4 |
| 4 | Button 5 |
