---
title: MICSAVE
slug: docs-sb3-micsave
system: SmileBASIC 3
type: command
category: Various kinds of input
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# MICSAVE

> **Category:** Various kinds of input

Copies data from the internal sampling memory to an array

## Format

```sb3
MICSAVE [[Acquisition position,] Number of samples to get,] Array name
```

## Arguments

| Argument | Description |
| --- | --- |
| `Acquisition<br>position` | Position to start capturing from (0-) |
| `Number of samples` | - Number of samples to capture (If omitted, the whole sampling buffer)<br>- Any value greater than the product of the sampling rate and the number of seconds specified<br>with MICSTART will give an error |
| `Array name` | - Array to store the captured sampling data<br>- For one-dimensional arrays, if the number of samples exceeds the number of elements, the<br>array will be extended automatically |

## Examples

```sb3
MICSTART 0,0,1 'rate:8180 bit:8 length:1sec
DIM WAVE%[8180] 'MICSIZE
MICSAVE 0,8180,WAVE%
```
