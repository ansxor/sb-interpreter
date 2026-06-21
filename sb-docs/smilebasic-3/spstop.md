---
title: SPSTOP
slug: docs-sb3-spstop
system: SmileBASIC 3
type: command
category: Sprites
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# SPSTOP

> **Category:** Sprites

Stops animation of a sprite If used before SPSET, an error will occur

## Format

```sb3
SPSTOP [Management number]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the target sprite: 0-511<br>* If the management number is omitted, animation of all sprites will be stopped. |

## Examples

```sb3
SPSTOP
```
