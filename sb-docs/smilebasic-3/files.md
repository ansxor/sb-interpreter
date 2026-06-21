---
title: FILES
slug: docs-sb3-files
system: SmileBASIC 3
type: command
category: Files
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# FILES

> **Category:** Files

## FILES (1)

Displays a file list on the console

### Format

```sb3
FILES ["File type"]
```

### Arguments

| Argument | Description |
| --- | --- |
| `File type` | To display only a certain type of file, specify the following:<br>"TXT:" Texts and programs<br>"DAT:" Binary data (including graphics)<br>"//" Project list<br>"PROJECT/" Project name should be specified |

### Examples

```sb3
FILES
```

## FILES (2)

Gets a file list and stores it in an array

### Format

```sb3
FILES ["File type",] String array
```

### Arguments

| Argument | Description |
| --- | --- |
| `File type` | To display only a certain type of file, specify the following:<br>"TXT:" Texts and programs<br>"DAT:" Binary data (including graphics)<br>"//" Project list<br>"PROJECT/" Project name should be specified |
| `String array` | String array to store the listed file names<br>- For one-dimensional arrays, the array will be automatically extended according to the number<br>of files obtained |

### Examples

```sb3
DIM NAMETBL$[100]
FILES NAMETBL$
```
