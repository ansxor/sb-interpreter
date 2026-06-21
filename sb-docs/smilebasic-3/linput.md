---
title: LINPUT
slug: docs-sb3-linput
system: SmileBASIC 3
type: command
category: Console input/output
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# LINPUT

> **Category:** Console input/output

Gets a character string input from the keyboard

- Also accepts "," and other characters that the INPUT instruction does not allow
- Waits for input until the ENTER key is input

## Format

```sb3
LINPUT ["Guiding text string";] String variable
```

## Arguments

| Argument | Description |
| --- | --- |
| `Guiding text<br>string` | Guidance message for input (Optional) |
| `String variable` | String variable to receive a single line input |

## Examples

```sb3
LINPUT "ADDRESS:";ADR$
```
