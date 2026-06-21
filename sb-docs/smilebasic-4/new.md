---
title: NEW
slug: docs-sb4-new
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-new
content_id: 19461
created: 2020-04-29
scraped: 2026-06-21
---

# NEW

Delete the code in the given program slot. If no program slot is given, all slots are cleared. Files are not edited or deleted. Direct Mode only.

## Syntax

```sbsyntax
NEW {slot}
```

| Input | Description |
| --- | --- |
| `slot` | The slot to clear (optional) |

## Examples

```sb4
NEW 1 'clear slot 1
NEW   'clear all slots
```
