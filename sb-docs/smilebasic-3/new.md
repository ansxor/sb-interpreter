---
title: NEW
slug: docs-sb3-new
system: SmileBASIC 3
type: command
category: Instructions available only in DIRECT mode
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# NEW

> **Category:** Instructions available only in DIRECT mode

Erases programs DIRECT mode only

## Format

```sb3
NEW [Program SLOT]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Program SLOT` | 0-3: Erases the specified SLOT only<br>If unspecified, all SLOTs are erased |

## Examples

```sb3
NEW
NEW 3
```
