---
title: CHKCHR
slug: docs-sb3-chkchr
system: SmileBASIC 3
type: command
category: Console input/output
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# CHKCHR

> **Category:** Console input/output

Checks the character code of a character on the console screen

## Format

```sb3
Variable = CHKCHR( X-coordinate,Y-coordinate )
```

## Arguments

| Argument | Description |
| --- | --- |
| `X-,Y-coordinates` | Coordinates in character units (X:0-49,Y:0-29) |

## Return Values

UTF-16 character code

## Examples

```sb3
CODE=CHKCHR(0,0)
```
