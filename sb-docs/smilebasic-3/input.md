---
title: INPUT
slug: docs-sb3-input
system: SmileBASIC 3
type: command
category: Console input/output
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# INPUT

> **Category:** Console input/output

Inputs numerical values or character strings from the keyboard

- Waits for input until the ENTER key is input
- If the number of input items is insufficient, "?Redo from start" will be displayed for re-input

## Format

```sb3
INPUT ["Guiding text string";] Variable[,Variable 2 …
]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Guiding text<br>string` | - Guidance message for input (Optional)<br>- If , (comma) is used instead of ; after the guiding text string, a ? mark will not be<br>displayed<br>- Only when ; is used, a string variable can be used for the guiding text string |
| `Variables` | - Variables to receive the input (numerical values or string variables)<br>- When specifying multiple variables, they should be delimited with commas (,) |

## Examples

```sb3
INPUT "Your name and age";NM$,AG
```
