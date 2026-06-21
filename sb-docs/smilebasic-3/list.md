---
title: LIST
slug: docs-sb3-list
system: SmileBASIC 3
type: command
category: Instructions available only in DIRECT mode
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# LIST

> **Category:** Instructions available only in DIRECT mode

Switches to EDIT mode and starts editing

- DIRECT mode only
- Using LIST with no argument is equal to pressing the EDIT button

## Format

```sb3
LIST [ Line number/ERR ]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Line number` | - If unspecified, the displayed list will start with the default line<br>- A program SLOT can be specified, e.g., by entering 2:120 |
| `ERR` | Specifies the line where the last error occurred |

## Examples

```sb3
LIST ERR
LIST 1:
```
