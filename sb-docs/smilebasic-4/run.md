---
title: RUN
slug: docs-sb4-run
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-run
content_id: 19468
created: 2020-04-29
scraped: 2026-06-21
---

# RUN

Run the program in the given slot. The memory of that slot (including its variables and `SPFUNC`/`TFUNC` mappings) is cleared before the program runs. Direct Mode only.

## Syntax

```sbsyntax
RUN [slot]
```

| Input | Description |
| --- | --- |
| `slot` | The program slot to run (optional, default `0`) |

## Examples

```sb4
RUN   'run the program in slot 0
RUN 2 ' run the program in slot 2
```
