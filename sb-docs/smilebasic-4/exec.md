---
title: EXEC
slug: docs-sb4-exec
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-exec
content_id: 19460
created: 2020-11-02
scraped: 2026-06-21
---

# EXEC

Run a program or slot.

## Syntax

```sbsyntax
EXEC program$, slot%
EXEC slot%
EXEC program$
```

| Input | Description |
| --- | --- |
| `program$` | The name/path of the program to load. |
| `slot%` | The program slot to use. |

If only `slot%` is specified, then the contents of the slot will be executed. If only `program$` is specified, then the program will be loaded into the currently-running slot, the running program will be replaced, and the new program will start running. If  both are specified, then the program will be loaded into the slot and the slot will be executed.

## Examples

```sb4
'run the program in slot 2
EXEC 2
```

```sb4
'load FOO into this slot and run it
EXEC "FOO"
```

```sb4
'load FOO into slot 1 and run it
EXEC "FOO",1
```
