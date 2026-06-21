---
title: CHKLABEL
slug: docs-sb3-chklabel
system: SmileBASIC 3
type: command
category: Basic instructions (data operations and others)
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# CHKLABEL

> **Category:** Basic instructions (data operations and others)

Checks if there is a label that can be referenced with the specified string

## Format

```sb3
Variable = CHKLABEL("@Label string"[,Flag])
```

## Arguments

| Argument | Description |
| --- | --- |
| `@Label string` | - It is also possible to check a different SLOT by using CHKLABEL "1:@Label name"<br>- The target SLOT should be enabled beforehand with USE, e.g., USE 1 |
| `Flag` | 0= Searches only within DEF (if omitted, 0)<br>1= If not found within DEF, searches for global labels |

## Return Values

```
FALSE= Does not exist, TRUE= Exists
```

## Examples

```sb3
A=CHKLABEL("@MAIN")
```
