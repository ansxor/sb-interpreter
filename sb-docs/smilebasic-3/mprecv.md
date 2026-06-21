---
title: MPRECV
slug: docs-sb3-mprecv
system: SmileBASIC 3
type: command
category: Wireless communication
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# MPRECV

> **Category:** Wireless communication

Receives data from MPSEND

- If there is no data to receive, the sender ID will contain the value -1
- Communication will be terminated if the system goes into sleep mode

## Format

```sb3
MPRECV OUT SID,RCV$
```

## Arguments

| Argument | Description |
| --- | --- |
| `SID` | 0-3: Connection destination number from which the string will be sent |
| `RCV$` | String variable to store the received data |

## Examples

```sb3
MPRECV OUT SID,RCV$
PRINT SID;":";RCV$
```
