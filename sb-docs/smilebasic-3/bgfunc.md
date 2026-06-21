---
title: BGFUNC
slug: docs-sb3-bgfunc
system: SmileBASIC 3
type: command
category: BG
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# BGFUNC

> **Category:** BG

Assigns a callback process to a BG layer

- An instruction for advanced users that is used when callback processing is required
- All BG layer processes are executed with CALL BG
- Instead of @Label, a user process defined using DEF can also be specified
- At the processing target, a management number can be obtained using a CALLIDX system variable

## Format

```sb3
BGFUNC Layer, @Label
```

## Arguments

| Argument | Description |
| --- | --- |
| `Layer` | Layer number: 0-3 |
| `@Label` | The label of the process target (or a user-defined process) |

## Examples

```sb3
BGFUNC 0,@LAYERSUB0
```
