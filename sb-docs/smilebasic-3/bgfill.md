---
title: BGFILL
slug: docs-sb3-bgfill
system: SmileBASIC 3
type: command
category: BG
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# BGFILL

> **Category:** BG

Fills the BG screen with a BG character

## Format

```sb3
BGFILL Layer,Start point X,Start point Y,End point X,End point Y,Screen data
```

## Arguments

| Argument | Description |
| --- | --- |
| `Layer` | Target layer number: 0-3 |
| `Start point X,Y` | Start point coordinates (Each coordinate: 0 - the value specified with the BGSCREEN<br>instruction minus 1) |
| `End point X,Y` | End point coordinates (Each coordinate: 0 - the value specified with the BGSCREEN instruction<br>minus 1) |
| `Screen Data` | \|b00\|<br>↑<br>\|<br>\| Character number (0-4095, repeated at the cycle of 1024)<br>\|b11\|<br>↓<br>\|b12\|<br>↑<br>Rotation by 90 degrees (Specified with two bits: b12 and b13)<br>\|b13\|<br>↓<br>[ 00 = 0 degrees, 01 = 90 degrees, 10 = 180 degrees, 11 = 270 degrees ]<br>\|b14\| Horizontal inversion (0=OFF, 1=ON)<br>\|b15\| Vertical inversion (0=OFF, 1=ON)<br>- 16-bit numerical value that specifies the character number and the rotation information<br>- A 4-digit hexadecimal string can also be specified ("0000"-"FFFF") |

## Examples

```sb3
BGFILL 0,0,0,19,15,1024
BGFILL 0,5,5,10,10,"C040"
```
