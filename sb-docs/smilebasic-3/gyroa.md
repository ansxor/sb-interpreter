---
title: GYROA
slug: docs-sb3-gyroa
system: SmileBASIC 3
type: command
category: Various kinds of input
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# GYROA

> **Category:** Various kinds of input

Gets information on the angle of the gyro sensor Motion sensor(s) should be enabled beforehand with XON MOTION

## Format

```sb3
GYROA OUT P,R,Y
```

## Arguments

| Argument | Description |
| --- | --- |
| `None` |  |

## Return Values

| Return Value | Description |
| --- | --- |
| `P` | Variable to receive Pitch (angle of the X-coordinate) (Unit: radian) |
| `R` | Variable to receive Roll (angle of the Y-coordinate) (Unit: radian) |
| `Y` | Variable to receive Yaw (angle of the Z-coordinate) (Unit: radian) |

## Examples

```sb3
XON MOTION
GYROA OUT P,R,Y
```
