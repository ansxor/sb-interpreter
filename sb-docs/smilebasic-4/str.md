---
title: STR$
slug: docs-sb4-str
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-str
content_id: 19494
created: 2020-11-11
scraped: 2026-06-21
---

# STR$

Convert a number to a string.

## Syntax

```sbsyntax
STR$ number# {, length% } OUT string$
```

| Input | Description |
| --- | --- |
| `number#` | An integer or real number to convert into a string. |
| `length%` | The *minimum* length of the returned string, in characters. Optional |

| Output | Description |
| --- | --- |
| `string$` | The string representation of `number#`. |

If `length%` is specified, the returned string is right-aligned with space characters until it is `length%` characters long. If the returned string's length is equal to or greater than `length%`, then it is not modified.

## Examples

```
VAR SCORE=10
PRINT "SCORE: "+STR$(SCORE)
```
