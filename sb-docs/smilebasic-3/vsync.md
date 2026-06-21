---
title: VSYNC
slug: docs-sb3-vsync
system: SmileBASIC 3
type: command
category: Basic instructions (data operations and others)
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# VSYNC

> **Category:** Basic instructions (data operations and others)

Stops the program until the specified number of vertically synchronized frames has been reached Unlike WAIT, the VSYNC count starts from the last VSYNC

## Format

```sb3
VSYNC [Number of frames]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Number of frames` | Specify the number of frames to wait, starting from the last VSYNC (0: Ignore; if omitted, 1<br>is assumed) |

## Examples

```sb3
VSYNC 1
```
