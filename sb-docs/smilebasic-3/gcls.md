---
title: GCLS
slug: docs-sb3-gcls
system: SmileBASIC 3
type: command
category: Graphics
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# GCLS

> **Category:** Graphics

Clears the graphic screen

- Instruction to fill the whole screen with black
- It is also possible to specify a color code with which to fill the screen

## Format

```sb3
GCLS [ Color code ]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Color code` | Color code consisting of an 8-bit value for each ARGB element * See GCOLOR |

## Examples

```sb3
GCLS RGB(32,32,32)
```
