---
title: MPSTART
slug: docs-sb3-mpstart
system: SmileBASIC 3
type: command
category: Wireless communication
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# MPSTART

> **Category:** Wireless communication

Starts a wireless communication session

- Connection to a session is allowed when MPSTART identifiers are equal
- The RESULT system variable should be used to get information on whether or not a session has successfully been

established

- Communication will be terminated if the system goes into sleep mode

## Format

```sb3
MPSTART Maximum number of connected users, "Communication identifier string"
```

## Arguments

| Argument | Description |
| --- | --- |
| `Maximum number of<br>connected users` | 2-4: Number of concurrent connected users |
| `Communication<br>identifier string` | Any character string for authentication |

## Examples

```sb3
MPSTART 4,"ANYSTR"
```
