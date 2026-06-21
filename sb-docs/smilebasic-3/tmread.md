---
title: TMREAD
slug: docs-sb3-tmread
system: SmileBASIC 3
type: command
category: Basic instructions (data operations and others)
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# TMREAD

> **Category:** Basic instructions (data operations and others)

Converts a time string to numerical values

## Format

```sb3
TMREAD ["Time string"] OUT H,M,S
```

## Arguments

| Argument | Description |
| --- | --- |
| `Time string` | Time string in "HH:MM:SS" format (if omitted, the current time) |

## Return Values

| Return Value | Description |
| --- | --- |
| `Variables to store<br>numerical values` | H: Variable to receive the hours (0-23)<br>M: Variable to receive the minutes<br>S: Variable to receive the seconds |

## Examples

```sb3
TMREAD "12:59:31" OUT H,M,S
```
