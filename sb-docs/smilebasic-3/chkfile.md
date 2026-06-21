---
title: CHKFILE
slug: docs-sb3-chkfile
system: SmileBASIC 3
type: command
category: Files
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# CHKFILE

> **Category:** Files

Checks if the specified file exists

## Format

```sb3
Variable = CHKFILE("[File type:]File name")
```

## Arguments

| Argument | Description |
| --- | --- |
| `File type` | "TXT:" Texts and programs<br>"DAT:" Binary data (including graphics) |
| `File name` | Name of the file to check |

## Return Values

```
TRUE= Exists, FALSE= Does not exist
```

## Examples

```sb3
A=CHKFILE("SBATTACK")
```
