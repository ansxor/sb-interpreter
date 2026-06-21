---
title: WAVSETA
slug: docs-sb3-wavseta
system: SmileBASIC 3
type: command
category: Sound
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# WAVSETA

> **Category:** Sound

Defines the sound of an MML user-defined musical instrument from an array

- Used for sound definition from an array obtained with MICSAVE
- 8180Hz sampling rate, 8 bits fixed

## Format

```sb3
WAVSETA Definition number,A,D,S,R,Numerical value array [,Reference pitch][,Start subscript][,End subscript]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Definition number` | - User-defined musical instrument number: 224-255<br>- This number is specified with the MML @ command |
| `A,D,S,R` | Envelope definition parameters<br>A: Attack (0-127)<br>D: Decay (0-127)<br>S: Sustain (0-127)<br>R: Release (0-127) |
| `Numerical value<br>array` | Array obtained with the MICSAVE instruction (Up to 16384 samples) |
| `Reference pitch` | If omitted, 69 (O4A) |
| `Start subscript` | Subscript of the element in the numerical value array at which to start reading (If omitted,<br>0) |
| `End subscript` | Subscript of the element in the numerical value array at which to stop reading (If omitted,<br>the last element) |

## Examples

```sb3
WAVSETA 224,0,95,100,20,SMPDATA
```
