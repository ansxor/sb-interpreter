---
title: EFCWET
slug: docs-sb3-efcwet
system: SmileBASIC 3
type: command
category: Sound
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# EFCWET

> **Category:** Sound

Sets the respective effect amounts for BEEP, BGM, and TALK

## Format

```sb3
EFCWET BEEP effect value, BGM effect value, TALK effect value
```

## Arguments

| Argument | Description |
| --- | --- |
| `BEEP effect value` | Effect amount for BEEP (0-127) |
| `BGM effect value` | Effect amount for BGM (0-127) |
| `TALK effect value` | - Effect setting for TALK (Less than 64: OFF; 64 or greater: ON)<br>- For TALK, the only available setting is ON/OFF; the amount does not change |

## Examples

```sb3
EFCWET 0,100,64
```
