---
title: LEFT$
slug: docs-sb4-left
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-left
content_id: 19485
created: 2020-10-27
scraped: 2026-06-21
---

# LEFT$

Copy a number of characters from the start of a string.

## Syntax

```sbsyntax
LEFT$ string$, length% OUT substring$
```

| Input | Output |
| --- | --- |
| `string$` | The string to copy characters from. |
| `length%` | The number of characters to copy. |

| Output | Description |
| --- | --- |
| `substring$` | A string consisting of the first `length%` characters of `string$`. |

## Examples

```sb4
'get the first word
PRINT LEFT$("Hello, world!",5)
```

```sb4
'this will be empty
PRINT LEFT$("abcde",0)
```
