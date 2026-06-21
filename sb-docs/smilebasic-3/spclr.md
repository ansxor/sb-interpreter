---
title: SPCLR
slug: docs-sb3-spclr
system: SmileBASIC 3
type: command
category: Sprites
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# SPCLR

> **Category:** Sprites

Stops using the specified sprite and releases the memory If memory is not released after use with sprites, there will be no available memory for SPSET

## Format

```sb3
SPCLR Management number
```

## Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the sprite to stop using: 0-511 |

## Examples

```sb3
SPCLR 56
```
