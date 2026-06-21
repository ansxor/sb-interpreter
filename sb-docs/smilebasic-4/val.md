---
title: VAL
slug: docs-sb4-val
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-val
content_id: 19495
created: 2020-11-11
scraped: 2026-06-21
---

# VAL

Convert a string into a number.

## Syntax

```sbsyntax
VAL string$ OUT number#
```

| Input | Description |
| --- | --- |
| `string$` | A string to convert into a number. |

| Output | Description |
| --- | --- |
| `number#` | The number represented by the contents of `string$`. |

The contents of `string$` must represent a legal SmileBASIC number literal of any type (integer, real, hex, binary, etc.) If the string does *not* represent a valid number, integer 0 is returned instead. The type of `number#` depends on the form of number contained in `string$`, e.g. `"&hFFFF"` will return an integer, `"123.45"` will return a real.

## Examples

```sb4
'integer literal
PRINT VAL("123")
```

```sb4
'hex literal
PRINT VAL("&hFFFF")
```

```sb4
'real literal
PRINT VAL("123.45")
```
