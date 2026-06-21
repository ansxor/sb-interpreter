---
title: BGLOAD
slug: docs-sb3-bgload
system: SmileBASIC 3
type: command
category: BG
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# BGLOAD

> **Category:** BG

Copies BG data from an array to the BG screen

## Format

```sb3
BGLOAD Layer, [Start point X,Start point Y,Width,Height,] Numerical value array
```

## Arguments

| Argument | Description |
| --- | --- |
| `Layer` | Layer number of the copy destination range: 0-3 |
| `Start point<br>X,Start point Y` | Start point coordinates (character coordinates) of the copy destination range |
| `Width, Height` | - Width and height (in character units) of the copy destination range<br>- If the range specification is omitted, the whole BG screen will be the display area. |
| `Numerical value<br>array` | Numerical value array containing the BG data stored with BGSAVE |

## Examples

```sb3
BGLOAD 0, 0,0,30,10, BGARRAY
```
