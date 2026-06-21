---
title: CHKCALL
slug: docs-sb3-chkcall
system: SmileBASIC 3
type: command
category: Basic instructions (data operations and others)
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# CHKCALL

> **Category:** Basic instructions (data operations and others)

Checks if there is an instruction or function that can be referenced with the specified string

## Format

```sb3
Variable = CHKCALL("Character string")
```

## Arguments

| Argument | Description |
| --- | --- |
| `Character string` | Character string of the instruction or function to check |

## Return Values

```
FALSE= Does not exist, TRUE= Exists
```

## Examples

```sb3
A=CHKCALL("KEYCHECK")
```
