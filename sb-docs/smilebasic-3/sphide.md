---
title: SPHIDE
slug: docs-sb3-sphide
system: SmileBASIC 3
type: command
category: Sprites
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# SPHIDE

> **Category:** Sprites

Hides a sprite

- This only hides the sprite; it continues to exist
- If used before SPSET, an error will occur

## Format

```sb3
SPHIDE Management number
```

## Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the sprite to hide: 0-511 |

## Examples

```sb3
SPHIDE 43
```
