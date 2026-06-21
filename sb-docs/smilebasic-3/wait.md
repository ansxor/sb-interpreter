---
title: WAIT
slug: docs-sb3-wait
system: SmileBASIC 3
type: command
category: Basic instructions (data operations and others)
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# WAIT

> **Category:** Basic instructions (data operations and others)

Stops the program until the specified number of vertically synchronized frames has been reached

## Format

```sb3
WAIT [Number of frames]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Number of frames` | Specify the number of frames to wait, starting from the present point (0: Ignore; if omitted,<br>1 is assumed) |

## Examples

```sb3
WAIT 60
```
