---
title: ATTR
slug: docs-sb3-attr
system: SmileBASIC 3
type: command
category: Console input/output
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# ATTR

> **Category:** Console input/output

Sets the rotation/inversion attributes of the characters to display on the console screen Constants for text attributes are available (#TROT0-270, #TREVH,V)

## Format

```sb3
ATTR Display attribute
```

## Arguments

| Argument | Description |
| --- | --- |
| `Display attribute` | ↑<br>\|b00\|<br>Rotation by 90 degrees (specified by using two bits: b00 and b01)<br>\|b01\|<br>↓<br>#TROT0, #TROT90, #TROT180, #TROT270<br>\|b02\| Horizontal inversion (0=OFF, 1=ON), #TREVH<br>\|b03\| Vertical inversion (0=OFF, 1=ON), #TREVV |

## Examples

```sb3
ATTR 3:PRINT "ABC"
```
