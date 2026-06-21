---
title: MICDATA
slug: docs-sb3-micdata
system: SmileBASIC 3
type: command
category: Various kinds of input
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# MICDATA

> **Category:** Various kinds of input

Gets data from the microphone This returns sampling data from the specified position

## Format

```sb3
Variable=MICDATA( Acquisition position )
```

## Arguments

| Argument | Description |
| --- | --- |
| `Acquisition<br>position` | - 0- (The range is determined according to the number of bits and the maximum number of<br>seconds)<br>- In loop mode, the range will not be checked |

## Return Values

| Return Value | Description |
| --- | --- |
| `Waveform data` | - For 8 bits, return values are 128-basis<br>- For 16 bits, return values are 32768-basis |

## Examples

```sb3
D=MICDATA(100)
```
