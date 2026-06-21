---
title: TOUCH
slug: docs-sb3-touch
system: SmileBASIC 3
type: command
category: Various kinds of input
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# TOUCH

> **Category:** Various kinds of input

Gets touch information The 5 pixels around the edge of the screen cannot be read

## Format

```sb3
TOUCH [Terminal ID] OUT STTM,TX,TY
```

## Arguments

| Argument | Description |
| --- | --- |
| `Terminal ID (0-3)` | This should be specified when information from another terminal is to be obtained via wireless<br>communication |

## Return Values

| Return Value | Description |
| --- | --- |
| `STTM` | Variable to receive the time when the screen is touched (0 = No touch) |
| `TX,TY` | - Variables to receive the touch coordinates (TX: 5-314, TY: 5-234)<br>- Note that returned values are not in the same range as the size of the Touch Screen |

## Examples

```sb3
TOUCH OUT TM,TX,TY
```
