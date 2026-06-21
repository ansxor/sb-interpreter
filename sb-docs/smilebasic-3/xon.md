---
title: XON
slug: docs-sb3-xon
system: SmileBASIC 3
type: command
category: Basic instructions (advanced control)
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# XON

> **Category:** Basic instructions (advanced control)

Declares the use of a special feature

- These features are not available unless their use is declared beforehand
- When XON EXPAD is successful, RESULT will be returned as TRUE.
- If the system is already in the XON state, this command will not display a dialog

## Format

```sb3
XON Name of feature to use
```

## Arguments

| Argument | Description |
| --- | --- |
| `Name of feature to<br>use` | MOTION: Motion sensor, gyro sensor<br>EXPAD: Circle Pad Pro<br>MIC: Microphone |

## Examples

```sb3
XON MOTION
```
