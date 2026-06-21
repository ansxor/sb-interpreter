---
title: SUBST$
slug: docs-sb4-subst
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-subst
content_id: 19488
created: 2020-10-27
scraped: 2026-06-21
---

# SUBST$

Create a new string by replacing a substring.

## Syntax

```sbsyntax
SUBST$ string$, startIndex% {, count% }, replacement$ OUT new$
```

| Input | Description |
| --- | --- |
| `string$` | The string to modify. |
| `startIndex%` | The index to start replacing characters at. |
| `count%` | The number of characters to replace (optional.)<br>If omitted, the rest of `string$` starting at `startIndex%` is replaced. |
| `replacement$` | The string used to replace the substring. |

| Output | Description |
| --- | --- |
| `new$` | The new string after the replacement is applied. |

## Examples

```sb4
'replace "brown fox" with "red cow"
PRINT SUBST$("The quick brown fox",10,"red cow")
```

```sb4
'replace "brown" with "red"
PRINT SUBST$("The quick brown fox",10,5,"red")
```
