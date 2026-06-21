---
title: SAVE
slug: docs-sb4-save
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-save
content_id: 19458
created: 2020-10-26
scraped: 2026-06-21
---

# SAVE

Save the contents of a program slot to a TXT file.

## Syntax

```sbsyntax
SAVE file$ {, slot% }
```

| Input | Description |
| --- | --- |
| `file$` | The name of the file to save. |
| `slot%` | The program slot to save (optional; default 0.) |

The `file$` parameter may optionally start with `TXT:`; e.g. `"TXT:FOO"` and `"FOO"` refer to the same file.
A destination project can be specified, e.g. `"BAR/FOO"` will save `TXT:FOO` in project `BAR/`. Otherwise, the file will be saved in the active project. However, only subprograms and UI programs have permission to save outside the active project.

## Examples

```sb4
'save slot 0 to TXT:FOO
SAVE "FOO"
SAVE "TXT:FOO"
SAVE "FOO",0
```

```sb4
'save slot 0 to TXT:FOO in BAR/
'only works in sub and UI!
SAVE "BAR/FOO"
```

## Notes

### Backup File

If `SAVE` overwrites an existing file, the previous contents are copied to `TXT:@BACKUP.PRG` in the destination project. If the backup file already exists, it is overwritten without being copied.

### `RESULT`

`SAVE` sets `RESULT` to 1 if the file is written successfully, 0 if the function fails for any reason, and -1 if the save is cancelled.
