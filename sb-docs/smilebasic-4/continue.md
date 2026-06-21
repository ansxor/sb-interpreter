---
title: CONTINUE
slug: docs-sb4-continue
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-continue
content_id: 19515
created: 2020-11-04
scraped: 2026-06-21
---

# CONTINUE

Skip to the next iteration of a loop.

## Syntax

```sbsyntax
CONTINUE
```

The `CONTINUE` keyword skips the current iteration of the loop it is within. It cannot be used outside of loops, a `Syntax error` will occur.

## Examples

```sb4
'don't print 13!
FOR I=1 TO 100
 IF I==13 THEN CONTINUE
 PRINT I
NEXT
```

## See Also

- [`FOR` loop](https://smilebasicsource.com/forum/thread/docs-sb4-for)
- [`WHILE` loop](https://smilebasicsource.com/forum/thread/docs-sb4-while)
- [`REPEAT` loop](https://smilebasicsource.com/forum/thread/docs-sb4-repeat)
- [`LOOP` loop](https://smilebasicsource.com/forum/thread/docs-sb4-loop)
