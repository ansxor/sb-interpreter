---
title: BGSCREEN
slug: docs-sb3-bgscreen
system: SmileBASIC 3
type: command
category: BG
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# BGSCREEN

> **Category:** BG

Sets the BG screen size per layer

## Format

```sb3
BGSCREEN Layer,Width,Height
```

## Arguments

| Argument | Description |
| --- | --- |
| `Layer` | Target layer number: 0-3 |
| `Width,Height` | - Width and height in character units (Width x Height should be equal to or less than 16383)<br>- Initial state: 25 x 15 (Right size to fill the upper screen with BG) |

## Examples

```sb3
BGSCREEN 0,128,127
```
