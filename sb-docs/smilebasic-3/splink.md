---
title: SPLINK
slug: docs-sb3-splink
system: SmileBASIC 3
type: command
category: Sprites
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# SPLINK

> **Category:** Sprites

Links one sprite to another sprite

- Only the coordinates will be linked (The rotation angle and magnification information will not)
- Only a sprite with a lower management number can be specified as the link destination (parent)
- The display coordinates of the child will be determined in relation to the parent
- In this coordinate system, the top left corner of the screen will not be the origin
- There are no restrictions on link hierarchies
- If used before SPSET, an error will occur

## Format

```sb3
SPLINK Management number, Link destination management number
```

## Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the link source (child) sprite: 0-511 |
| `Link destination<br>management number` | Management number of the link destination (parent) sprite: 0-511<br>* Management numbers that are not lower than that of the link source will give errors. |

## Examples

```sb3
SPLINK 15,4
```
