---
title: SCROLL
slug: docs-sb3-scroll
system: SmileBASIC 3
type: command
category: Console input/output
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# SCROLL

> **Category:** Console input/output

Adjusts the display location of the whole console screen

- Can give the impression of a moving view point (characters will move in the opposite direction)
- Characters pushed out of the screen will disappear

## Format

```sb3
SCROLL Number of characters X, Number of characters Y
```

## Arguments

| Argument | Description |
| --- | --- |
| `Number of<br>characters X` | Amount of horizontal view point movement (Negative values indicate leftward movement, positive<br>values rightward movement) |
| `Number of<br>characters Y` | Amount of vertical view point movement (Negative values indicate upward movement, positive<br>values downward movement) |

## Examples

```sb3
SCROLL 5,7
```
