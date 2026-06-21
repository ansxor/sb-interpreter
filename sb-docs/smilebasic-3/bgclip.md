---
title: BGCLIP
slug: docs-sb3-bgclip
system: SmileBASIC 3
type: command
category: BG
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# BGCLIP

> **Category:** BG

Specifies the display area of the BG screen

## Format

```sb3
BGCLIP Layer [,Starting point X,Starting point Y,End point X,End point Y]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Layer` | Target layer number: 0-3 |
| `Start point X,Y` | Start point coordinates (in pixels) of the display area |
| `End point X,Y` | - End point coordinates (in pixels) of the display area<br>- If the start and end points are omitted, the whole layer will be the display area |

## Examples

```sb3
BGCLIP 0,20,20,379,219
```
