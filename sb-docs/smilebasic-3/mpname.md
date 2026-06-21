---
title: MPNAME$
slug: docs-sb3-mpname
system: SmileBASIC 3
type: command
category: Wireless communication
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# MPNAME$

> **Category:** Wireless communication

Gets the terminal name of a specified terminal in a wireless communication session Communication will be terminated if the system goes into sleep mode

## Format

```sb3
String variable = MPNAME$( Terminal ID )
```

## Arguments

| Argument | Description |
| --- | --- |
| `Terminal ID` | 0-3: ID of another terminal in the wireless communication session |

## Return Values

Terminal name string

## Examples

```sb3
NAME$=MPNAME$( 3 )
```
