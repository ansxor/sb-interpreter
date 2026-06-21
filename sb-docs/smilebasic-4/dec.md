---
title: DEC
slug: docs-sb4-dec
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-dec
content_id: 19470
created: 2020-11-03
scraped: 2026-06-21
---

# DEC

Decrement a variable.

## Syntax

```sbsyntax
DEC variable# {, decr# }
```

| Input | Description |
| --- | --- |
| `variable#` | The integer or real number variable to modify. |
| `decr#` | The value to use for the increment.<br>Optional. 1, if omitted. |

## Examples

```sb4
'decrement I
DEC I
```

```sb4
'decrement A by 3.1
DEC A,3.1
```

## Notes

### Integer Overflow/Underflow

If `DEC` will cause an integer value to overflow or underflow, the value is first converted to a real number.

```sb4
'underflow test
VAR I%=&h80000000  'minimum int
DEC I%
??I%  'now a real
```
