---
title: WIDTH
slug: docs-sb3-width
system: SmileBASIC 3
type: command
category: Console input/output
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# WIDTH

> **Category:** Console input/output

Changes the console character sizes

- Only enlarges the characters, does not display a smooth zoomed-in view
- This is an auxiliary function for people who have trouble viewing small characters

## Format

```sb3
WIDTH Font size
```

## Arguments

| Argument | Description |
| --- | --- |
| `Font size` | 8: 8x8 pixels (Standard)<br>16: 16x16 pixels (Twice as large as the normal horizontal and vertical display) |

## Examples

```sb3
WIDTH 16
A=WIDTH()
```
