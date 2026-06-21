---
title: OPTION
slug: docs-sb3-option
system: SmileBASIC 3
type: command
category: Basic instructions (data operations and others)
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# OPTION

> **Category:** Basic instructions (data operations and others)

Sets the operating mode of the program

## Format

```sb3
OPTION Feature name
```

## Arguments

| Argument | Description |
| --- | --- |
| `Feature name` | STRICT: Variable declaration is required (A reference without declaration will give an error)<br>DEFINT: Causes the default variable type to be Integer |

## Examples

```sb3
OPTION STRICT
```
