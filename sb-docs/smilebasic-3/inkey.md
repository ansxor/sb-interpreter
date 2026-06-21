---
title: INKEY$
slug: docs-sb3-inkey
system: SmileBASIC 3
type: command
category: Console input/output
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# INKEY$

> **Category:** Console input/output

Gets a character input from the keyboard (without waiting for input)

## Format

```sb3
String variable=INKEY$()
```

## Arguments

| Argument | Description |
| --- | --- |
| `None` |  |

## Return Values

```
- A character (UTF-16) from the keyboard
- If there is no input, "" will be returned
```

## Examples

```sb3
C$=INKEY$()
```
