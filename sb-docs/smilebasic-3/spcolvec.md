---
title: SPCOLVEC
slug: docs-sb3-spcolvec
system: SmileBASIC 3
type: command
category: Sprites
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# SPCOLVEC

> **Category:** Sprites

Sets a movement speed for sprite collision detection

- It is recommended to also call this instruction when setting SPCOL
- If used before SPSET, an error will occur

## Format

```sb3
SPCOLVEC Management number [,Movement amount X,Movement amount Y]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the target sprite: 0-511 |
| `Movement amount<br>X,Movement amount<br>Y` | - If omitted, the amount will be automatically calculated as follows:<br>- When linear interpolation of "XY" in SPANIM is being performed: Movement distance from the<br>previous frame<br>- Otherwise: 0,0 |

## Examples

```sb3
SPCOLVEC 93
```
