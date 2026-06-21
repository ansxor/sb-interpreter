---
title: MPGET
slug: docs-sb3-mpget
system: SmileBASIC 3
type: command
category: Wireless communication
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# MPGET

> **Category:** Wireless communication

Gets user-defined data from a specified terminal in a wireless communication session Communication will be terminated if the system goes into sleep mode

## Format

```sb3
Variable=MPGET( Terminal ID, Internal management number )
```

## Arguments

| Argument | Description |
| --- | --- |
| `Terminal ID` | 0-3: ID of another terminal in the wireless communication session |
| `Internal<br>management number` | 0-8: Management number of the target data |

## Return Values

```
Numerical value (integer) of the specified data
```

## Examples

```sb3
RET=MPGET( 0, 5 )
```
