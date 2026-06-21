---
title: INC
slug: docs-sb4-inc
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-inc
content_id: 19469
created: 2020-10-26
scraped: 2026-06-21
---

# INC

Increment a variable / append to a string.

## Syntax

```sbsyntax
INC var {, incr }
```

| Input | Description |
| --- | --- |
| `var` | The variable to modify. |
| `incr` | The value to use for the increment. |

`INC` can be used for both numbers and strings. When `var` is a number, its value is increased by the value of `incr`; e.g. `INC FOO,2` will increase `FOO` by 2. `incr` is optional in this case; if omitted, it defaults to 1. When `var` is a string, `incr` must be a string, and cannot be omitted. The value of `incr` is appended to `var`; e.g. if `FOO` is `"ABC"`, then `INC FOO,"DE"` will result in `FOO` being `"ABCDE"`.

## Examples

```sb4
'increment I
INC I
```

```sb4
'increment A by 3.1
INC A,3.1
```

```sb4
'append "FOO" to A$
VAR A$="BAR"
INC A$,"FOO"
??A$  '>BARFOO
```

## Notes

### Integer Overflow/Underflow

If `INC` will cause an integer value to overflow or underflow, the value is first converted to a real number.

```sb4
'overflow test
VAR I%=&h7FFFFFFF  'maximum int
INC I%
??I%  'now a real
```
