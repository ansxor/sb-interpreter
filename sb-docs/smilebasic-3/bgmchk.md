---
title: BGMCHK
slug: docs-sb3-bgmchk
system: SmileBASIC 3
type: command
category: Sound
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# BGMCHK

> **Category:** Sound

Checks music playback status

## Format

```sb3
Variable=BGMCHK( [Track number] )
```

## Arguments

| Argument | Description |
| --- | --- |
| `Track number` | Track number: 0-7 (If omitted, 0) |

## Return Values

```
FALSE = Stopped, TRUE = Playing
```

## Examples

```sb3
RET=BGMCHK(0)
```
