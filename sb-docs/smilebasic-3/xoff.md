---
title: XOFF
slug: docs-sb3-xoff
system: SmileBASIC 3
type: command
category: Basic instructions (advanced control)
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# XOFF

> **Category:** Basic instructions (advanced control)

Stops using a special feature declared with XON

## Format

```sb3
XOFF Name of the feature to stop
```

## Arguments

| Argument | Description |
| --- | --- |
| `Name of feature to<br>stop` | MOTION: Motion sensor, gyro sensor<br>EXPAD: Circle Pad Pro<br>MIC: Microphone |

## Examples

```sb3
XOFF MOTION
```
