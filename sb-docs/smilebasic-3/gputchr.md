---
title: GPUTCHR
slug: docs-sb3-gputchr
system: SmileBASIC 3
type: command
category: Graphics
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# GPUTCHR

> **Category:** Graphics

## GPUTCHR (1)

Draws a character string on the graphic screen

### Format

```sb3
GPUTCHR X,Y, "String" [,Scale X,Scale Y][,Color code]
```

### Arguments

| Argument | Description |
| --- | --- |
| `X,Y` | Display position (X: 0-399, Y: 0-239) |
| `"String"` | String to display |
| `Scale X,Y` | Display magnification (No scaling=1.0) |
| `Color code` | Color code consisting of an 8-bit value for each ARGB element * See GCOLOR |

### Examples

```sb3
GPUTCHR 10,10,"
"
```

## GPUTCHR (2)

Draws a character on the graphic screen

### Format

```sb3
GPUTCHR X,Y, Character code [,Scale X,Scale Y][,Color code]
```

### Arguments

| Argument | Description |
| --- | --- |
| `X,Y` | Display position (X: 0-399, Y: 0-239) |
| `Character code` | Character code to display |
| `Scale X,Y` | Display magnification (No scaling=1.0) |
| `Color code` | Color code consisting of an 8-bit value for each ARGB element * See GCOLOR |

### Examples

```sb3
GPUTCHR 10,10,ASC("A")
```
