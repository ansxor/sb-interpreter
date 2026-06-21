---
title: BREAK
slug: docs-sb4-break
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-break
content_id: 19514
created: 2020-11-04
scraped: 2026-06-21
---

# BREAK

Break out of a loop.

## Syntax

```sbsyntax
BREAK
```

The `BREAK` keyword causes the loop it is within to exit immediately. It can *only* occur inside of loops; using it outside of one is a syntax error.

## Examples

```sb4
LOOP
 IF !RND(10) THEN BREAK
ENDLOOP
```

## See Also

- [`FOR` loop](https://smilebasicsource.com/forum/thread/docs-sb4-for)
- [`WHILE` loop](https://smilebasicsource.com/forum/thread/docs-sb4-while)
- [`REPEAT` loop](https://smilebasicsource.com/forum/thread/docs-sb4-repeat)
- [`LOOP` loop](https://smilebasicsource.com/forum/thread/docs-sb4-loop)
