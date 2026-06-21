---
title: TMREAD
slug: docs-sb4-tmread
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-tmread
content_id: 19503
created: 2020-10-28
scraped: 2026-06-21
---

# TMREAD

Decompose a timestamp string to numbers.

## Syntax

```sbsyntax
TMREAD { timestamp$ } OUT hour%, minute%, second%
```

| Input | Description |
| --- | --- |
| `timestamp$` | The timestamp to convert. (optional)<br>If omitted, the value of `TIME$` is used. |

| Output | Description |
| --- | --- |
| `hour%` | The hour contained in `timestamp$`. |
| `minute%` | The minute contained in `timestamp$`. |
| `second%` | The second contained in `timestamp$`. |

## Examples

```sb4
'read the current time
VAR HR%,MN%,SC%
TMREAD OUT HR%,MN%,SC%
```

```sb4
'read the given timestamp
VAR HR%,MN%,SC%
TMREAD "01:23:45" OUT HR%,MN%,SC%
```
