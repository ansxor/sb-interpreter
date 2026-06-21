---
title: SPUNLINK
slug: docs-sb3-spunlink
system: SmileBASIC 3
type: command
category: Sprites
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# SPUNLINK

> **Category:** Sprites

Unlinks a sprite If used before SPSET, an error will occur

## Format

```sb3
SPUNLINK Management number
```

## Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the sprite to unlink: 0-511 |

## Examples

```sb3
SPUNLINK 15
```
