---
title: CHKVAR
slug: docs-sb3-chkvar
system: SmileBASIC 3
type: command
category: Basic instructions (data operations and others)
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# CHKVAR

> **Category:** Basic instructions (data operations and others)

Checks if there is a variable that can be referenced with the specified string

## Format

```sb3
Variable = CHKVAR("Character string")
```

## Arguments

| Argument | Description |
| --- | --- |
| `Character string` | Character string of the variable to check |

## Return Values

```
FALSE= Does not exist, TRUE= Exists
```

## Examples

```sb3
A=CHKVAR("COUNTX")
```
