---
title: FLOAT
slug: docs-sb4-float
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-float
content_id: 19499
created: 2021-01-14
scraped: 2026-06-21
---

# FLOAT

Convert a numeric value to a floating-point (real number) type. If the value is already a float it is unchanged.

> To round a real-number value, use `FLOOR`, `CEIL`, or `ROUND`.

## Syntax

```sbsyntax
FLOAT number% out float#
```

| Input | Description |
| --- | --- |
| `number%` | Number to convert to float. |

| Output | Description |
| --- | --- |
| `float#` | `number%` converted to a float. |

## Examples

```sb4
INSPECT FLOAT(100)
```
