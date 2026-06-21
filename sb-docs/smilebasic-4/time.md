---
title: TIME$
slug: docs-sb4-time
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-time
content_id: 19501
created: 2020-10-28
scraped: 2026-06-21
---

# TIME$

Return the current time as a string.

## Syntax

```sbsyntax
TIME$ OUT timestamp$
```

| Output | Description |
| --- | --- |
| `timestamp$` | The current time in a string, formatted as `Hr:Mn:Sc`. |

The time is always returned in 24-hour format.

## Examples

```sb4
'print the current time
'e.g. at 1:23:45 AM
PRINT TIME$()  '01:23:45
```
