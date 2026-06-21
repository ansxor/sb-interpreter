---
title: BGMVOL
slug: docs-sb3-bgmvol
system: SmileBASIC 3
type: command
category: Sound
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# BGMVOL

> **Category:** Sound

Sets the volume for the specified track

## Format

```sb3
BGMVOL [Track number,] Volume
```

## Arguments

| Argument | Description |
| --- | --- |
| `Track number` | Target track number: 0-7 (If omitted, 0) |
| `Volume` | Volume level to set: 0-127 |

## Examples

```sb3
BGMVOL 0,64
```
