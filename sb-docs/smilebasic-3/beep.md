---
title: BEEP
slug: docs-sb3-beep
system: SmileBASIC 3
type: command
category: Sound
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# BEEP

> **Category:** Sound

Generates a simple alarm sound or sound effect

## Format

```sb3
BEEP [Sound effect number][,Frequency][,Volume][,Pan pot]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Sound effect<br>number` | - Type of sound to generate: Preset sound 0-133<br>- A list of preset sounds can be viewed by pressing the SMILE button |
| `Frequency` | - Frequency value to change to: -32768 to 32767 (One halftone per 100) |
| `Volume` | - Volume level for playback: 0-127 |
| `Pan pot` | - Stereo pan pot specification: 0 (Left) - 64 (Center) - 127 (Right) |

## Examples

```sb3
BEEP 20
```
