---
title: BGSAVE
slug: docs-sb3-bgsave
system: SmileBASIC 3
type: command
category: BG
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# BGSAVE

> **Category:** BG

Copies the contents of the BG screen to a numerical value array

## Format

```sb3
BGSAVE Layer, [Start point X,Start point Y,Width,Height,] Numerical value array
```

## Arguments

| Argument | Description |
| --- | --- |
| `Layer` | Layer number of the copy source: 0-3 |
| `Start point<br>X,Start point Y` | Start point coordinates (character coordinates) of the copy source range |
| `Width, Height` | - Width and height (in character units) of the copy source range<br>- If the range specification is omitted, the whole BG screen will be the display area |
| `Numerical value<br>array` | - Numerical value array to which to copy the data<br>- For one-dimensional arrays only, if the array is insufficient, the required element(s) will<br>be added automatically |

## Examples

```sb3
DIM BGARRAY[30*10]
BGSAVE 0, 0,0,30,10, BGARRAY
```
