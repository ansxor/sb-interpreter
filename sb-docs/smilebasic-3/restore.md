---
title: RESTORE
slug: docs-sb3-restore
system: SmileBASIC 3
type: command
category: Basic instructions (data operations and others)
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# RESTORE

> **Category:** Basic instructions (data operations and others)

Specifies the first DATA to read with the READ instruction

## Format

```sb3
RESTORE @Label
```

## Arguments

| Argument | Description |
| --- | --- |
| `@Label` | - @Label name given to the beginning of the DATA instruction to be read<br>- A string variable to which a @Label name is assigned can also be specified<br>- It is also possible to reference a label from a different SLOT by using the format RESTORE<br>"1:@Label name"<br>- The target SLOT should be enabled beforehand with USE, e.g., USE 1 |

## Examples

```sb3
RESTORE @DATATOP
@DATATOP
DATA 123,345,56,"SAMPLE"
```
