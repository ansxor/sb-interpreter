---
title: PRGSET
slug: docs-sb3-prgset
system: SmileBASIC 3
type: command
category: Source code manipulation
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# PRGSET

> **Category:** Source code manipulation

Substitutes the contents of the current line with the specified string If PRGGET$ has returned an empty string, a line will be added

## Format

```sb3
PRGSET "Character string"
```

## Arguments

| Argument | Description |
| --- | --- |
| `Character string` | Character string to substitute the current line with |

## Examples

```sb3
PRGSET "'Comment"
```
