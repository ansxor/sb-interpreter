---
title: INSTR
slug: docs-sb4-instr
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-instr
content_id: 19482
created: 2020-10-27
scraped: 2026-06-21
---

# INSTR

Search for an occurrence of a substring within a string.

## Syntax

```sbsyntax
INSTR { startIndex% }, string$, substring$ OUT index%
```

| Input | Description |
| --- | --- |
| `startIndex%` | The index within `string$` to start searching from. Optional, default 0. |
| `string$` | The string to search within. |
| `substring$` | The substring to search for. |

| Output | Description |
| --- | --- |
| `index%` | The index of the substring found within `string$`. -1 if no occurrence is found |

`INSTR` will return only the index of the first occurrence of `substring$` it finds, starting from the `startIndex%`. If `starIndex%` is omitted, then the search starts from the beginning of `string$` (index 0). If `substring$` is not found, then `index%` will be -1.

## Examples

```sb4
'find the letter 'a' within this string.
PRINT INSTR("The quick brown fox jumps over the lazy dog.","a")  '36
```

```sb4
'find the first "chuck" starting at character 40
CONST #CHUCK="How much wood could a woodchuck chuck if a woodchuck could chuck wood?"
INSTR 40,#CHUCK,"chuck" OUT I%
PRINT I%  '47
```

```sb4
'this match will fail
PRINT INSTR("######","!")  '-1
```

## Notes

### Empty Substring

If `substring$` is empty, *every* index in any `string$` will match it.

```sb4
'these all match
PRINT INSTR("Hello, world!","")    '0
PRINT INSTR(3,"Hello, world!","")  '3
PRINT INSTR(7,"Hello, world!","")  '7
```

An interesting quirk of this is that the empty string will match at one index past the end of the string.

```sb4
'3 is one past the end of the string
PRINT INSTR(3,"ABC","")  '3
PRINT INSTR(4,"ABC","")  '-1
```
