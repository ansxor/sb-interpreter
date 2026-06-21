---
title: STICK
slug: docs-sb3-stick
system: SmileBASIC 3
type: command
category: Various kinds of input
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# STICK

> **Category:** Various kinds of input

Gets information on the Circle Pad

## Format

```sb3
STICK [Terminal ID] OUT X,Y
```

## Arguments

| Argument | Description |
| --- | --- |
| `Terminal ID (0-3)` | This should be specified when information is obtained from another terminal via wireless<br>communication |

## Return Values

| Return Value | Description |
| --- | --- |
| `X,Y` | - Variables to receive Circle Pad input magnitude ( X:±1.0, Y:±1.0 )<br>- Actual return values will be around ±0.86<br>- For Y values,<br>↑<br>corresponds to positive and<br>↓<br>to negative |

## Examples

```sb3
STICK OUT X,Y
STICK 3 OUT X,Y
```
