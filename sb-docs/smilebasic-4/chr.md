---
title: CHR$
slug: docs-sb4-chr
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-chr
content_id: 19483
created: 2020-10-27
scraped: 2026-06-21
---

# CHR$

Get a character given its character code.

```sbsyntax
CHR$ charCode% OUT character$
```

| Input | Description |
| --- | --- |
| `charCode%` | The character code of the character (0-65535). |

| Output | Description |
| --- | --- |
| `character$` | The character corresponding to `charCode%`, as a string. |

If `charCode%` is out of range, then it is wrapped around; e.g. -1 is 65535, 65536 is 0, etc.

## Examples

```sb4
'get the "A" character (65)
PRINT CHR$(65)  'A
```
