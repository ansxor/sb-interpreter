---
title: BGCOPY
slug: docs-sb3-bgcopy
system: SmileBASIC 3
type: command
category: BG
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# BGCOPY

> **Category:** BG

Copies from the BG screen in character units

## Format

```sb3
BGCOPY Layer,Start point X,Start point Y, End point X,End point Y, Transfer destination X,Transfer destination Y
```

## Arguments

| Argument | Description |
| --- | --- |
| `Layer` | Target layer number: 0-3 |
| `Start point X,Y<br>End point X,Y` | Start and End point coordinates of the copy source (0 - the value specified with BGSCREEN<br>minus 1) |
| `Transfer<br>destination X,Y` | Start point coordinates of the copy destination (0 - the value specified with BGSCREEN minus<br>1) |

## Examples

```sb3
BGCOPY 2,0,0,32,32,0,0
```
