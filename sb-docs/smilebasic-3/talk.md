---
title: TALK
slug: docs-sb3-talk
system: SmileBASIC 3
type: command
category: Sound
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# TALK

> **Category:** Sound

Generates synthesized speech Alphanumeric symbols are read out character-by-character

## Format

```sb3
TALK "Voice string"
```

## Arguments

| Argument | Description |
| --- | --- |
| `Voice string` | Synthesized speech string (Characters will be read out directly) |

## Special commands

```
A special command enclosed with <> is available for use in strings
<S Speed>: Speech speed (Speed: 0-65536, default: 32768)
<P Pitch>: Tone pitch (Pitch: 0-65536, default: 32768)
```

## Examples

```sb3
TALK "ABCDE<P50000><S20000>FGHIJKLM"
```
