---
title: WAVSET
slug: docs-sb3-wavset
system: SmileBASIC 3
type: command
category: Sound
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# WAVSET

> **Category:** Sound

Defines the sound of an MML user-defined musical instrument

## Format

```sb3
WAVSET Definition number,A,D,S,R,"Waveform string" [,Reference pitch]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Definition number` | - User-defined musical instrument number: 224-255<br>- This number is specified with the MML @ command. |
| `A,D,S,R` | Envelope definition parameters<br>A: Attack (0-127)<br>D: Decay (0-127)<br>S: Sustain (0-127)<br>R: Release (0-127) |
| `Waveform string` | - Hexadecimal string<br>- Two characters represent one sample value (8 bits)<br>- &H00 - &H80 (128) - &HFF (255)<br>- 16, 32, 64, 128, 256, or 512 samples can be specified<br>- The number of characters should be twice the number of samples |
| `Reference pitch` | If omitted, 69 (O4A) |

## Examples

```sb3
W$="7F7F7F7FFFFFFFFF7F7F7F7FFFFFFFFF"*4
WAVSET 224,3,10,30,5,W$,69
```
