---
title: KEY
slug: docs-sb3-key
system: SmileBASIC 3
type: command
category: Basic instructions (data operations and others)
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# KEY

> **Category:** Basic instructions (data operations and others)

Assigns an arbitrary character string to a function key

## Format

```sb3
KEY Number,"Character string"
```

## Arguments

| Argument | Description |
| --- | --- |
| `Number` | Number of the function key (1-5) |
| `Character string` | - Character string to assign<br>- If the whole string cannot be displayed, '<br>…<br>' will be displayed at the end |

## Examples

```sb3
KEY 1,"CLS"+CHR$(13)
```
