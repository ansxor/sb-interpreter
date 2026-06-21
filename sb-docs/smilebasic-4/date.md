---
title: DATE$
slug: docs-sb4-date
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-date
content_id: 19502
created: 2020-10-28
scraped: 2026-06-21
---

# DATE$

Return the current date as a string.

## Syntax

```sbsyntax
DATE$ OUT datestamp$
```

| Output | Description |
| --- | --- |
| `datestamp$` | The current date in a string, formatted as `Year/Mo/Dy`. |

## Examples

```sb4
'print the current date
'e.g. on January 2nd 2019
PRINT DATE$()  '2019/01/02
```
