---
title: DTREAD
slug: docs-sb3-dtread
system: SmileBASIC 3
type: command
category: Basic instructions (data operations and others)
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# DTREAD

> **Category:** Basic instructions (data operations and others)

Converts a date string to numerical values

## Format

```sb3
DTREAD ["Date string"] OUT Y,M,D [,W]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Date string` | Date string in "YYYY/MM/DD" format (if omitted, the current date and time) |

## Return Values

| Return Value | Description |
| --- | --- |
| `Variables to store<br>numerical values` | Y: Variable to receive the year<br>M: Variable to receive the month<br>D: Variable to receive the day<br>W: Variable to receive the day of the week (numerical value: 0 for Sunday) |

## Examples

```sb3
DTREAD "2014/10/12" OUT Y,M,D
```
