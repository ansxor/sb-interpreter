---
title: TYPEOF
slug: docs-sb4-typeof
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-typeof
content_id: 19473
created: 2020-11-04
scraped: 2026-06-21
---

# TYPEOF

Check the type of a value.

## Syntax

```sbsyntax
TYPEOF value OUT type%
```

| Input | Description |
| --- | --- |
| `value` | The value whose type to check. |

| Output | Description |
| --- | --- |
| `type%` | The type of the value. — Value: 0, Constant: `#T_DEFAULT`, Description: Default type; Value: 1, Constant: `#T_INT`, Description: Integer; Value: 2, Constant: `#T_REAL`, Description: Real number; Value: 3, Constant: `#T_STR`, Description: String; Value: 5, Constant: `#T_INTARRAY`, Description: Integer array; Value: 6, Constant: `#T_REALARRAY`, Description: Real number array; Value: 7, Constant: `#T_STRARRAY`, Description: String array |

The default type `#T_DEFAULT` (also called "empty") is a special value used when arguments or return values from a function are unset.

## Examples

```sb4
'check type of this number
PRINT TYPEOF(0)  '1
```

```sb4
'check type of this variable
VAR S$="ABC"
PRINT TYPEOF(S$)  '3
```

```sb4
'demonstration of empty value
'the second argument to TEST is not passed
DEF TEST A,B
 PRINT TYPEOF(A)
 PRINT TYPEOF(B)
END

TEST 1,
```
