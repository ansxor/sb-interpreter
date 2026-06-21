---
title: SPFUNC
slug: docs-sb3-spfunc
system: SmileBASIC 3
type: command
category: Sprites
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# SPFUNC

> **Category:** Sprites

Assigns a process to a sprite

- An instruction for advanced users that is used when callback processing is required
- All sprite processes are executed with CALL sprite
- Instead of @Label, a user process defined using DEF can also be specified
- At the processing target, the management number can be obtained using the CALLIDX system variable
- If used before SPSET, an error will occur

## Format

```sb3
SPFUNC Management number, @Label
```

## Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the target sprite: 0-511 |
| `@Label` | Label of the processing target (or a user-defined process) to be called |

## Examples

```sb3
SPFUNC 0,@PROG
```
