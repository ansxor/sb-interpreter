---
title: INT
slug: docs-sb4-int
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-int
content_id: 19498
created: 2021-01-14
scraped: 2026-06-21
---

# INT

Convert a numeric value to an integer type. The fractional portion is truncated (rounded toward zero.) If the value is already an integer it is unchanged.

> To round a real-number value, use `FLOOR`, `CEIL`, or `ROUND`.

## Syntax

```sbsyntax
INT number# out integer%
```

| Input | Description |
| --- | --- |
| `number#` | Number to convert to integer. |

| Output | Description |
| --- | --- |
| `integer%` | `number#` converted to an integer. |

## Examples

```sb4
PRINT INT(100.456)  '=>  100
```
