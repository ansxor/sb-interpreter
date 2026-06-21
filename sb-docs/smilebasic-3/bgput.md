---
title: BGPUT
slug: docs-sb3-bgput
system: SmileBASIC 3
type: command
category: BG
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# BGPUT

> **Category:** BG

Places a BG character on the BG screen No image will be displayed for character number 0

## Format

```sb3
BGPUT Layer,X,Y,Screen data
```

## Arguments

| Argument | Description |
| --- | --- |
| `Layer` | Target layer number: 0-3 |
| `X,Y` | Coordinates to place the character at (0 - the value specified with BGSCREEN minus 1) |
| `Screen Data` | \|b00\|<br>↑<br>\|   \| Character number (0-4095, repeated at the cycle of 1024)<br>\|b11\|<br>↓<br>\|b12\|<br>↑<br>Rotation by 90 degrees (Specified with two bits: b12 and b13)<br>\|b13\|<br>↓<br>[ 00 = 0 degrees, 01 = 90 degrees, 10 = 180 degrees, 11 = 270 degrees ]<br>\|b14\| Horizontal inversion (0=OFF, 1=ON)<br>\|b15\| Vertical inversion (0=OFF, 1=ON)<br>- 16-bit numerical value that specifies the character number and the rotation information<br>- A 4-digit hexadecimal string can also be specified ("0000"-"FFFF") |

## Examples

```sb3
BGPUT 0,0,0,5
BGPUT 0,20,15,"80FF"
```
