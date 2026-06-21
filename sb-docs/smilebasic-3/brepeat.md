---
title: BREPEAT
slug: docs-sb3-brepeat
system: SmileBASIC 3
type: command
category: Various kinds of input
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# BREPEAT

> **Category:** Various kinds of input

Sets the key repeat feature

- Omitting Start time and Interval will turn off repeat
- Management numbers differ from the bit values that correspond to each button in BUTTON
- ZR and ZL buttons are available only when Circle Pad Pro is used

## Format

```sb3
BREPEAT Button ID, Start time, Interval
```

## Arguments

| Argument | Description |
| --- | --- |
| `Button ID` | 0: +Control Pad up ID<br>1: +Control Pad down ID<br>2: +Control Pad left ID<br>3: +Control Pad right ID<br>4: A button ID<br>5: B button ID<br>6: X button ID<br>7: Y button ID<br>8: L button ID<br>9: R button ID<br>10: Not used<br>11: ZR button ID<br>12: ZL button ID |
| `Start time` | Time from when a key is pressed first to when repeat begins (in units of 1/60th of a second) |
| `Interval` | Repeat interval after repeat begins (in units of 1/60th of a second, 0 = Repeat OFF) |

## Examples

```sb3
BREPEAT 0,15,4
```
