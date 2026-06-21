---
title: LOCATE
slug: docs-sb3-locate
system: SmileBASIC 3
type: command
category: Console input/output
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# LOCATE

> **Category:** Console input/output

Specifies the character display location on the console screen

## Format

```sb3
LOCATE [X-coordinate],[Y-coordinate] [,Z-coordinate]
```

## Arguments

| Argument | Description |
| --- | --- |
| `X-,Y-coordinates` | - Coordinates of each character (X:0-49,Y:0-29)<br>- If the X- and Y-coordinates are omitted, the previous coordinates for each will be kept |
| `Z-coordinate` | - Coordinate in the depth direction (Rear:1024<Screen surface:0<Front:-256)<br>- If omitted, the previous Z-coordinate will be kept |

## Examples

```sb3
LOCATE 20,15
LOCATE 0,0,-200
```
