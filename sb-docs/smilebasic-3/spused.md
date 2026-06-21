---
title: SPUSED
slug: docs-sb3-spused
system: SmileBASIC 3
type: command
category: Sprites
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# SPUSED

> **Category:** Sprites

Checks if the specified sprite is in use

## Format

```sb3
Variable=SPUSED(Management number)
```

## Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the target sprite: 0-511 |

## Return Values

```
TRUE = In use, FALSE = Available
```

## Examples

```sb3
S=SPUSED(4)
```
