---
title: STOP
slug: docs-sb4-stop
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-stop
content_id: 19511
created: 2020-11-04
scraped: 2026-06-21
---

# STOP

Suspend the program, optionally displaying a message.

## Syntax

```sbsyntax
STOP { message$ }
```

| Input | Description |
| --- | --- |
| `message$` | A message to print on the console after stopping. Optional. |

`STOP` will suspend the program execution and print a message to the text console. `STOP` will always first print `Break on slot:line`, where `slot` and `line` are replaced by the running program slot and the line number of the `STOP` statement. If `message$` is specified, it will be printed after. If the program is running from the Top Menu, a message prompting the user to press A or Enter to quit is also displayed. In Direct Mode, entering `CONT` will resume the program, unless another program is loaded, an error has occurred, etc.

## Examples

```sb4
STOP
STOP "Good bye!"
```
