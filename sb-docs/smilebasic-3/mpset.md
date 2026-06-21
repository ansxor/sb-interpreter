---
title: MPSET
slug: docs-sb3-mpset
system: SmileBASIC 3
type: command
category: Wireless communication
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# MPSET

> **Category:** Wireless communication

Writes to user-defined data in a wireless communication session Communication will be terminated if the system goes into sleep mode

## Format

```sb3
MPSET Internal management number, Numerical value
```

## Arguments

| Argument | Description |
| --- | --- |
| `Internal<br>management number` | 0-8: Management number of the target data/td> |
| `Numerical value` | Numerical value to register (Only an integer value is allowed) |

## Examples

```sb3
MPSET 5,123
```
