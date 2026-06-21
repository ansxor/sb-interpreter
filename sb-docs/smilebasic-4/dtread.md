---
title: DTREAD
slug: docs-sb4-dtread
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-dtread
content_id: 19504
created: 2020-10-28
scraped: 2026-06-21
---

# DTREAD

Decompose a datestamp string to numbers.

## Syntax

```sbsyntax
DTREAD { datestamp$ } OUT year%, month%, day% {, weekday% }
```

| Input | Description |
| --- | --- |
| `datestamp$` | The datestamp to convert. (optional)<br>If omitted, the value of `DATE$` is used. |

| Output | Description |
| --- | --- |
| `year%` | The year contained in `datestamp$`. |
| `month%` | The month contained in `datestamp$`. |
| `day%` | The day contained in `datestamp$`. |
| `weekday%` | The day of the week corresponding to the date. (optional)<br>0-6, Sunday is 0. |

## Examples

```sb4
'read the current date
VAR YR%,MO%,DY%
DTREAD OUT YR%,MO%,DY%
```

```sb4
'what weekday is this date?
VAR WK%
DTREAD "2019/12/25" OUT ,,,WK%
```
