---
title: RENAME
slug: docs-sb3-rename
system: SmileBASIC 3
type: command
category: Files
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# RENAME

> **Category:** Files

Changes a file name When run, a confirmation dialog will be displayed

## Format

```sb3
RENAME "[File type:]File name", "[File type:]New name"
```

## Arguments

| Argument | Description |
| --- | --- |
| `File type:` | "TXT:" Texts and programs (optional)<br>"DAT:" Binary data (including graphics) |
| `File name` | Name of file to change name of |
| `New name` | New file name |

## Examples

```sb3
RENAME "SAMPLE","NEWNAME"
```
