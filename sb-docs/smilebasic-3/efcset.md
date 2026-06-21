---
title: EFCSET
slug: docs-sb3-efcset
system: SmileBASIC 3
type: command
category: Sound
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# EFCSET

> **Category:** Sound

## EFCSET (1)

Selects a music effect type

### Format

```sb3
EFCSET Type number
```

### Arguments

| Argument | Description |
| --- | --- |
| `Type number` | 0: No effect (Same as EFCOFF)<br>1: Reverb (Bathroom)<br>2: Reverb (Cave)<br>3: Reverb (Space) |

### Examples

```sb3
EFCSET 2
```

## EFCSET (2)

Sets effect parameters (For advanced users)

### Format

```sb3
EFCSET Initial reflection time,Reverberation sound delay time,Reverberation sound decay time,Reverberation sound
filter coefficient 1,Reverberation sound filter coefficient 2,Initial reflection sound gain,Reverberation sound
gain
```

### Arguments

| Argument | Description |
| --- | --- |
| `Initial reflection<br>time` | 0-2000 (msec) |
| `Reverberation<br>sound delay time` | 0-2000 (msec) |
| `Reverberation<br>sound decay time` | 1-10000 (msec) |
| `Reverberation<br>sound filter<br>coefficient 1` | 0.0-1.0 |
| `Reverberation<br>sound filter<br>coefficient 2` | 0.0-1.0 |
| `Initial reflection<br>sound gain` | 0.0-1.0 |
| `Reverberation<br>sound gain` | 0.0-1.0 |

### Examples

```sb3
EFCSET 997,113,1265,0.1,0,0.2,0.1
```
