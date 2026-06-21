---
title: BGGET
slug: docs-sb3-bgget
system: SmileBASIC 3
type: command
category: BG
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# BGGET

> **Category:** BG

Gets information on a BG character on the BG screen

## Format

```sb3
Variable=BGGET( Layer, X, Y [,Coordinate system flag] )
```

## Arguments

| Argument | Description |
| --- | --- |
| `Layer` | Target layer number: 0-3 |
| `X,Y` | Coordinates to get the BG character from (Coordinate values differ depending on the coordinate<br>system flag described below) |
| `Coordinate system<br>flag (If omitted,<br>0)` | 0: Treats X-, Y-coordinates as the BG screen coordinates (in character units)<br>1: Treats X-, Y-coordinates as the screen coordinates (in pixel units) |

## Return Values

```
|b00|
↑
|
| Character number (0-4095, repeated at cycles of 1024)
|b11|
↓
|b12|
↑
Rotation by 90 degrees (Specified with two bits: b12 and b13)
|b13|
↓
#BGROT0, #BGROT90, #BGROT0180, #BGROT270
|b14| Horizontal inversion (0=OFF, 1=ON), #BGREVH
|b15| Vertical inversion (0=OFF, 1=ON), #BGREVV
Screen data
```

## Examples

```sb3
C=BGGET(0,12,14)
```
