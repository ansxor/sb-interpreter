---
title: SPSTART
slug: docs-sb3-spstart
system: SmileBASIC 3
type: command
category: Sprites
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# SPSTART

> **Category:** Sprites

Starts animation of a sprite (If used before SPSET, an error will occur)

## Format

```sb3
SPSTART [Management number]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the target sprite: 0-511<br>* If the management number is omitted, animation of all sprites will be started. |

## Examples

```sb3
SPSTART
```
