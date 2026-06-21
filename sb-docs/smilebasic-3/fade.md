---
title: FADE
slug: docs-sb3-fade
system: SmileBASIC 3
type: command
category: Screen control
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# FADE

> **Category:** Screen control

## FADE (1)

Sets the color for the screen fader

- The fader is always displayed in the front
- The entire screen is filled with the fading color (taking the transparent color into consideration)

### Format

```sb3
FADE Fading color [,Fading time]
```

### Arguments

| Argument | Description |
| --- | --- |
| `Fading color` | The color to fill the screen (specifying RGB(0,0,0,0) disables the fader) |
| `Fading time` | The screen color changes from the current fading color to the specified fading color over a<br>specified time period, which can be specified in units of 1/60th of a second. |

### Examples

```sb3
FADE RGB(32,64,64,64),60
```

## FADE (2)

Gets the current screen fader color

### Format

```sb3
Value=FADE()
```

### Return Values

Color code consisting of an 8-bit value for each ARGB element

### Examples

```sb3
C=FADE()
```
