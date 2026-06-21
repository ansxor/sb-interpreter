---
title: STICKEX
slug: docs-sb3-stickex
system: SmileBASIC 3
type: command
category: Various kinds of input
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# STICKEX

> **Category:** Various kinds of input

Gets information on the Circle Pad Pro stick Circle Pad Pro should be enabled beforehand with XON EXPAD

## Format

```sb3
STICKEX [Terminal ID] OUT X,Y
```

## Arguments

| Argument | Description |
| --- | --- |
| `Terminal ID (0-3)` | This should be specified when information from another terminal is to be obtained via wireless<br>communication |

## Return Values

| Return Value | Description |
| --- | --- |
| `X,Y` | Variables to receive Circle Pad Pro input magnitude ( X:±1.0, Y:±1.0 ) |

## Examples

```sb3
XON EXPAD
STICKEX OUT X,Y
```
