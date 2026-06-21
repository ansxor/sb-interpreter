---
title: MID$
slug: docs-sb4-mid
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-mid
content_id: 19487
created: 2020-10-27
scraped: 2026-06-21
---

# MID$

Copy a substring out of a string.

## Syntax

```sbsyntax
MID$ string$, startIndex% {, length% } OUT substring$
```

| Input | Description |
| --- | --- |
| `string$` | The string to copy from. |
| `startIndex%` | The index in `string$` to start copying from. |
| `length%` | The number of characters to copy (optional.)<br>If omitted, copy characters up to the end of `string$`. |

| Output | Description |
| --- | --- |
| `substring$` | The substring copied from `string$`. |

## Examples

```sb4
'copy the word "brown"
PRINT MID$("The quick brown fox",10,5)
```

```sb4
'copy the alphabet starting at N
PRINT MID$("ABCDEFGHIJKLMNOPQRSTUVWXYZ",13)
```
