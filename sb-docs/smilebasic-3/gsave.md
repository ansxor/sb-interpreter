---
title: GSAVE
slug: docs-sb3-gsave
system: SmileBASIC 3
type: command
category: Graphics
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# GSAVE

> **Category:** Graphics

Copies an image (whole screen) to an array

## Format

```sb3
GSAVE [Transfer source page,] [X,Y,Width,Height,] Transfer destination array, Color conversion flag
```

## Arguments

| Argument | Description |
| --- | --- |
| `Transfer source<br>page` | 0-5 (GRP0-GRP5), -1(GRPF) If omitted: Current drawing page |
| `X,Y,Width,Height` | Start point X-coordinate, start point Y-coordinate, and width/height (in pixels) of the copy<br>source range<br>If omitted: Current drawing area |
| `Transfer<br>destination array` | Array variable to store the image<br>* If the number of elements in the array is insufficient, the required element(s) will be<br>added automatically, provided that the array is one-dimensional. |
| `Color conversion<br>flag` | 0: Performs color conversion (Converts to 32-bit logical colors)<br>1: Leaves the physical codes as they are (16-bit) |

## Examples

```sb3
DIM WORK[0]
GSAVE 0,0,0,512,512,WORK,1
```
