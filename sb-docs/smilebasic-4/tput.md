---
title: TPUT
slug: docs-sb4-tput
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-tput
content_id: 19547
created: 2020-11-02
scraped: 2026-06-21
---

# TPUT

Put a character on a text screen at the specified coordinates.

## Syntax

```sbsyntax
TPUT screenID%, x%, y%, charCode% {, attribute% }
TPUT screenID%, x%, y%, string$ {, attribute% }
```

| Input | Description |
| --- | --- |
| `screenID%` | ID of the target text screen. |
| `x%`, `y%` | Coordinates where to write the character. |
| `charCode%` | Character code of the character to write. |
| `string$` | Character to write as a string. |
| `attribute%` | Display attribute bitset to apply to the character.<br>Optional. If unspecified, the attributes set by `ATTR` are used. |

If `string$` is used, only the first character of the string is written.

## Examples

```sb4
'put "A" on text screen 0 at 10,10
TPUT 0, 10, 10, "A"
```

```sb4
'put an upside-down user character
TPUT 0, 10, 10, #TUSRCHR+10, #TREVV
```
