---
title: GCOPY
slug: docs-sb3-gcopy
system: SmileBASIC 3
type: command
category: Graphics
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# GCOPY

> **Category:** Graphics

Copies an image from another graphic page

## Format

```sb3
GCOPY [Transfer source page,] Start point X,Start point Y, End point X,End point Y, Transfer destination
X,Transfer destination Y, Copy mode
```

## Arguments

| Argument | Description |
| --- | --- |
| `Transfer source<br>page` | 0-5 (GRP0-GRP5), -1 (GRPF) If omitted: Current drawing page |
| `Start point X,Y<br>End point X,Y` | Start and end point coordinates of the copy source range (X: 0-399, Y: 0-239) |
| `Transfer<br>destination X,Y` | Start point coordinates of the copy destination range (X: 0-399, Y: 0-239) |
| `Copy mode` | TRUE = Copies the transparent color, FALSE = Does not copy the transparent color |

## Examples

```sb3
GCOPY 0, 0,0,100,100, 200,100 ,1
```
