---
title: GYROSYNC
slug: docs-sb3-gyrosync
system: SmileBASIC 3
type: command
category: Various kinds of input
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# GYROSYNC

> **Category:** Various kinds of input

Updates gyro information

- Error accumulation may occur if gyro information is repeatedly retrieved
- This instruction should be called to reset information appropriately
- However, calling this instruction at an interval of 1 frame or less is prohibited

## Format

```sb3
GYROSYNC
```

## Examples

```sb3
GYROSYNC
```
