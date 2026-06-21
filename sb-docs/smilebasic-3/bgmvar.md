---
title: BGMVAR
slug: docs-sb3-bgmvar
system: SmileBASIC 3
type: command
category: Sound
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# BGMVAR

> **Category:** Sound

## BGMVAR (1)

Writes to an MML internal variable

### Format

```sb3
BGMVAR Track number, Variable number, Value
```

### Arguments

| Argument | Description |
| --- | --- |
| `Track number` | Target MML track number: 0-7 |
| `Variable number` | Internal variable to which to write a value: 0-7 ($0-$7 in MML) |
| `Value` | Value to write to the variable |

### Examples

```sb3
BGMVAR 0,5,10
```

## BGMVAR (2)

Reads an MML internal variable

### Format

```sb3
Variable=BGMVAR(Track number, Variable number )
```

### Arguments

| Argument | Description |
| --- | --- |
| `Track number` | Target MML track number: 0-7 |
| `Variable number` | Internal variable from which to read the value: 0-7 ($0-$7 in MML) |

### Return Values

```
Content of the specified variable during playback (When the music is stopped, -1)
```

### Examples

```sb3
MC=BGMVAR(0,5)
```
