---
title: MPSEND
slug: docs-sb3-mpsend
system: SmileBASIC 3
type: command
category: Wireless communication
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# MPSEND

> **Category:** Wireless communication

Sends data to all participants in a wireless communication session

- Delivery of sent data is guaranteed, but with a delay
- A large number of MPSEND calls in a short period will result in an error

  * Communication buffer overflow

- Communication will be terminated if the system goes into sleep mode

## Format

```sb3
MPSEND "Character string to send"
```

## Arguments

| Argument | Description |
| --- | --- |
| `Character string<br>to send` | Character string of up to 256 bytes |

## Examples

```sb3
MPSEND "HELLO!"
```
