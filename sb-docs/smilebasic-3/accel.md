---
title: ACCEL
slug: docs-sb3-accel
system: SmileBASIC 3
type: command
category: Various kinds of input
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# ACCEL

> **Category:** Various kinds of input

Gets information on acceleration

- The motion sensor should be enabled beforehand with XON MOTION
- Note that this instruction will continue to detect 1G acceleration in the gravity direction
- This is useful when operation is performed while tilting

## Format

```sb3
ACCEL OUT X,Y,Z
```

## Arguments

| Argument | Description |
| --- | --- |
| `None` |  |

## Return Values

| Return Value | Description |
| --- | --- |
| `X,Y,Z` | Variables to receive acceleration (Unit: G) |

## Examples

```sb3
XON MOTION
ACCEL OUT X,Y,Z
```
