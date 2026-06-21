---
title: BGCOORD
slug: docs-sb3-bgcoord
system: SmileBASIC 3
type: command
category: BG
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# BGCOORD

> **Category:** BG

Converts display coordinates to BG screen coordinates, or vice versa

## Format

```sb3
BGCOORD Layer,Source X-coordinate,Source Y-coordinate[,Mode]OUT DX,DY
```

## Arguments

| Argument | Description |
| --- | --- |
| `Layer` | Layer number: 0-3 |
| `Source X-, Y-<br>coordinates` | Coordinates to convert (BG character coordinates or display coordinates) |
| `Mode` | Conversion mode: 0-2<br>0: Converts BG screen coordinates to display coordinates<br>1: Converts display coordinates to BG screen coordinates (in character units)<br>2: Converts display coordinates to BG screen coordinates (in pixel units) |
| `DX,DY` | Variable to store the converted coordinates (BG character coordinates or display coordinates) |

## Examples

```sb3
BGCOORD 0,BGX,BGY,0 OUT DX,DY
```
