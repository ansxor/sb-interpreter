---
title: RIGHT$
slug: docs-sb4-right
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-right
content_id: 19486
created: 2020-10-27
scraped: 2026-06-21
---

# RIGHT$

Copy a number of characters from the end ("right side") of a string.

## Syntax

```sbsyntax
RIGHT$ string$, length% OUT substring$
```

| Input | Output |
| --- | --- |
| `string$` | The string to copy characters from. |
| `length%` | The number of characters to copy. |

| Output | Description |
| --- | --- |
| `substring$` | A string consisting of the last `length%` characters of `string$`. |

## Examples

```sb4
'get the exclamation
PRINT RIGHT$("Hello, world!",6)
```

```sb4
'this will be empty
PRINT RIGHT$("abcde",0)
```
