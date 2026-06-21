---
title: BGMSTOP
slug: docs-sb3-bgmstop
system: SmileBASIC 3
type: command
category: Sound
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# BGMSTOP

> **Category:** Sound

## BGMSTOP (1)

Stops playing music

### Format

```sb3
BGMSTOP [Track number [,Fading time]]
```

### Arguments

| Argument | Description |
| --- | --- |
| `Track number` | Target track number: 0-7 (If omitted, all tracks will be stopped) |
| `Fading time` | Seconds (Decimal fractions are allowed; 0 = Stop immediately; if omitted, handled as 0) |

### Examples

```sb3
BGMSTOP
```

## BGMSTOP (2)

Stops playing music

- Forces ongoing sounds such as release sounds to stop
- Executing this will cause user-defined BGM 255 to be overwritten

### Format

```sb3
BGMSTOP -1
```

### Arguments

| Argument | Description |
| --- | --- |
| `-1: Value for forcibly stopping sound` |  |

### Examples

```sb3
BGMSTOP -1
```
