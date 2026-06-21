---
title: ASC
slug: docs-sb4-asc
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-asc
content_id: 19484
created: 2020-10-27
scraped: 2026-06-21
---

# ASC

Get the character code of a character.

```sbsyntax
ASC character$ OUT charCode%
```

| Input | Description |
| --- | --- |
| `character$` | The character you want to convert, as a string. |

| Output | Description |
| --- | --- |
| `charCode%` | The character code of `character$` (0-65535). |

If `character$` is more than one character long, only the first character of the string is converted.

## Examples

```sb4
'get the charcode of "A" (65)
PRINT ASC("A")  '65
```

```sb4
'only the first character is counted (a)
PRINT ASC("abcde")  '97
```

## Notes

### Can't Use Empty String

An empty string contains no characters, so `ASC` cannot be used on one.

```sb4
PRINT ASC("")
'Can't use empty string in 0:1(ASC:1)
```

### Meaning of ASC

The name `ASC` comes from [ASCII](https://en.wikipedia.org/wiki/ASCII), the most common standard character encoding before [Unicode](https://en.wikipedia.org/wiki/Unicode), and the basis of Unicode and many others.
