---
title: GYROV
slug: docs-sb3-gyrov
system: SmileBASIC 3
type: command
category: Various kinds of input
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# GYROV

> **Category:** Various kinds of input

Gets information on the angular velocity of the gyro sensor Motion sensor(s) should be enabled beforehand with XON MOTION

## Format

```sb3
GYROV OUT P,R,Y
```

## Arguments

| Argument | Description |
| --- | --- |
| `None` |  |

## Return Values

| Return Value | Description |
| --- | --- |
| `P` | Variable to receive Pitch (angular velocity of the X-coordinate) (Unit: radians/second) |
| `R` | Variable to receive Roll (angular velocity of the Y-coordinate) (Unit: radians/second) |
| `Y` | Variable to receive Yaw (angular velocity of the Z-coordinate) (Unit: radians/second) |

## Examples

```sb3
XON MOTION
GYROV OUT P,R,Y
```
