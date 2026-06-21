---
title: BUTTON
slug: docs-sb3-button
system: SmileBASIC 3
type: command
category: Various kinds of input
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# BUTTON

> **Category:** Various kinds of input

Gets the status of hardware buttons Constants for buttons are available for return values

## Format

```sb3
Variable=BUTTON( [Feature ID [,Terminal ID]] )
```

## Arguments

| Argument | Description |
| --- | --- |
| `Feature ID` | 0: Held down<br>1: Moment pressed (with the repeat feature enabled)<br>2: Moment pressed (with the repeat feature disabled)<br>3: Moment released |
| `Terminal ID (0-3)` | This should be specified to get information from another terminal via wireless communication |

## Return Values

```
|b00| +Control Pad up (1), #UP
|b01| +Control Pad down (2), #DOWN
|b02| +Control Pad left (4), #LEFT
|b03| +Control Pad right (8), #RIGHT
|b04| A button (16), #A
|b05| B button (32), #B
|b06| X button (64), #X
|b07| Y button (128), #Y
|b08| L button (256), #L
|b09| R button (512), #R
|b10| Not used
|b11| ZR button (2048), #ZL
|b12| ZL button (4096), #ZR
- The buttons correspond to b0-b12 (If a button is pressed, its corresponding bit = 1)
- Contents in () next to button names are decimal numerals
- ZR and ZL buttons are available only when Circle Pad Pro is used
```

## Examples

```sb3
B=BUTTON()
B=BUTTON( 0,3 )
```
