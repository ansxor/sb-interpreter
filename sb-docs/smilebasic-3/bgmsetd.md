---
title: BGMSETD
slug: docs-sb3-bgmsetd
system: SmileBASIC 3
type: command
category: Sound
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# BGMSETD

> **Category:** Sound

Predefines a user-defined tune

- The DATA instruction should be used for internal registration of MML ( DATA "CDEFGAB" )
- The end of DATA is determined according to the numerical value ( DATA 0 )
- Internally, this is handled in the same way as RESTORE
- RESTORE must be used to READ the data after BGMSETD
- Executing immediately after BGMPLAY will cause a delay of approx. 2 frames

## Format

```sb3
BGMSETD User-defined tune number,"@Label string"
```

## Arguments

| Argument | Description |
| --- | --- |
| `User-defined tune<br>number` | User-defined tune number: 128-255 |
| `@Label string` | - A label string where an MML string has been registered with DATA<br>- Should be specified by enclosing the string in " or by assigning it to a string variable<br>- Pressing the Help button for "MML" will display the description of MML commands |

## Examples

```sb3
BGMSETD 128,"@MMLTOP"
```
