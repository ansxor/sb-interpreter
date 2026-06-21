---
title: BGMSET
slug: docs-sb3-bgmset
system: SmileBASIC 3
type: command
category: Sound
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# BGMSET

> **Category:** Sound

Predefines a user-defined piece of music Executing immediately after BGMPLAY will cause a delay of approx. 2 frames

## Format

```sb3
BGMSET User-defined tune number,"MML string"
```

## Arguments

| Argument | Description |
| --- | --- |
| `User-defined tune<br>number` | User-defined tune number: 128-255 |

## MML string

Pressing the Help button for "MML" will display the description of MML commands

## Examples

```sb3
BGMSET 128,"CDEFG"
```
