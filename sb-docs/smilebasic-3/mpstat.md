---
title: MPSTAT
slug: docs-sb3-mpstat
system: SmileBASIC 3
type: command
category: Wireless communication
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# MPSTAT

> **Category:** Wireless communication

Gets the connection status of a specified terminal in a wireless communication session Communication will be terminated if the system goes into sleep mode

## Format

```sb3
Variable = MPSTAT( [Terminal ID] )
```

## Arguments

| Argument | Description |
| --- | --- |
| `Terminal ID` | 0-3: ID of another terminal in the wireless communication session (If omitted, the whole<br>session will be assumed) |

## Return Values

0: Not connected, 1: Connected

## Examples

```sb3
RET=MPSTAT( 2 )
```
